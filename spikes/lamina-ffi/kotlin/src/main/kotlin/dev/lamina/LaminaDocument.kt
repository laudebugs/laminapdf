package dev.lamina

import dev.lamina.ffi.FfiCancelToken
import dev.lamina.ffi.FfiDocument
import dev.lamina.ffi.FfiException
import dev.lamina.ffi.FfiInfoKey
import dev.lamina.ffi.FfiPageInfo
import dev.lamina.ffi.FfiSaveMode
import dev.lamina.ffi.FfiSaveOptions
import dev.lamina.ffi.FfiSaveStep
import dev.lamina.ffi.FfiSaved
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.NonCancellable
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.asExecutor
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.ensureActive
import kotlinx.coroutines.isActive
import kotlinx.coroutines.withContext
import java.nio.ByteBuffer

/**
 * The hand-written idiomatic layer over the generated bindings (ADR 0029's rule: generated
 * code is internal, this is the API). Blocking calls run on [dispatcher]; long work is a job
 * stepped in a loop that checks the coroutine between steps, and cancelling the coroutine
 * cancels the Rust job through its token, so a step already running stops at its next check.
 */
class LaminaDocument private constructor(
    /** The generated handle: internal, so callers go through this layer (ADR 0029 §4). */
    internal val raw: FfiDocument,
    private val dispatcher: CoroutineDispatcher,
) : AutoCloseable {
    companion object {
        /**
         * Opens a document from a [ByteArray]. Convenient, but it copies twice (into a
         * `RustBuffer`, then into Rust's own `Vec`); for large files use the direct
         * [ByteBuffer] overload, which Rust reads in place and copies once.
         */
        suspend fun open(
            bytes: ByteArray,
            password: String? = null,
            dispatcher: CoroutineDispatcher = Dispatchers.IO,
        ): LaminaDocument = withContext(dispatcher) { LaminaDocument(FfiDocument.open(bytes, password), dispatcher) }

        /**
         * Opens from a direct [ByteBuffer] (UniFFI 0.32's zero-copy `&[u8]`): the bytes from its
         * position to its limit are read in place for the call; Rust copies them once to own
         * them. The generated code reads from the buffer's base address, so the layer passes a
         * [ByteBuffer.slice] that starts at the position.
         */
        suspend fun open(
            direct: ByteBuffer,
            password: String? = null,
            dispatcher: CoroutineDispatcher = Dispatchers.IO,
        ): LaminaDocument =
            withContext(dispatcher) { LaminaDocument(FfiDocument.openBorrowed(direct.slice(), password), dispatcher) }
    }

    suspend fun pageCount(): Int = withContext(dispatcher) { raw.pageCount().toInt() }

    suspend fun pages(): List<FfiPageInfo> = withContext(dispatcher) { raw.pageInfos() }

    suspend fun title(): String? = withContext(dispatcher) { raw.info().title }

    suspend fun setTitle(title: String?) = withContext(dispatcher) { raw.setInfo(FfiInfoKey.TITLE, title) }

    /**
     * Saves as a job of [unitsPerStep] units per step, reporting progress. Cancelling the
     * calling coroutine cancels the Rust job: the step in flight returns `cancelled` at its
     * next check, and this throws [CancellationException]. If that step had already finished
     * the save, Rust has rebased the document onto the output, and a cancelled coroutine can't
     * return it: this call's [onSavedAfterCancel] receives it instead, so the bytes are never
     * lost. Per call, so overlapping saves can't take each other's bytes.
     */
    suspend fun save(
        mode: FfiSaveMode = FfiSaveMode.AUTO,
        unitsPerStep: UInt = 256u,
        onSavedAfterCancel: (FfiSaved) -> Unit,
        onProgress: (FfiSaveStep.Progress) -> Unit = {},
    ): FfiSaved {
        val job = withContext(dispatcher) { raw.saveJob(FfiSaveOptions(mode, false)) }
        job.use {
            while (true) {
                val step =
                    cancellableStep(
                        dispatcher,
                        onCancel = job::cancel,
                        onResultAfterCancel = { if (it is FfiSaveStep.Done) onSavedAfterCancel(it.saved) },
                    ) { job.step(unitsPerStep) }
                when (step) {
                    is FfiSaveStep.Done -> {
                        // Cancelled after the last step returned: a cancelled coroutine's return
                        // value is discarded, so hand the bytes over here too.
                        if (!currentCoroutineContext().isActive) onSavedAfterCancel(step.saved)
                        return step.saved
                    }
                    is FfiSaveStep.Progress -> onProgress(step)
                }
            }
        }
    }

    /** A run-to-completion save with a token (the blocking form, for comparison). */
    fun saveBlocking(mode: FfiSaveMode, token: FfiCancelToken = FfiCancelToken()): FfiSaved =
        raw.save(FfiSaveOptions(mode, false), token)

    override fun close() = raw.close()
}

/**
 * Runs one blocking Rust call on [dispatcher] and suspends until it returns. If the coroutine
 * is cancelled meanwhile, [onCancel] runs at once (cancelling the Rust job through its token),
 * then this waits, non-cancellably, for the call in flight (at most one step), hands a value it
 * returned to [onResultAfterCancel] so it is never dropped, and throws [CancellationException].
 * A `cancelled` error from Rust is rethrown as [CancellationException].
 */
suspend fun <T> cancellableStep(
    dispatcher: CoroutineDispatcher,
    onCancel: () -> Unit,
    onResultAfterCancel: (T) -> Unit = {},
    block: () -> T,
): T {
    currentCoroutineContext().ensureActive() // cancelled between steps: don't start another
    val pending = CompletableDeferred<Result<T>>()
    dispatcher.asExecutor().execute { pending.complete(runCatching(block)) }
    val result =
        try {
            pending.await()
        } catch (e: CancellationException) {
            onCancel()
            withContext(NonCancellable) { pending.await() }.onSuccess(onResultAfterCancel)
            throw e
        }
    return result.getOrElse { e ->
        if (e is FfiException.Lamina && e.code == "cancelled") {
            throw CancellationException("lamina job cancelled").apply { initCause(e) }
        }
        throw e
    }
}

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
import kotlinx.coroutines.withContext
import java.nio.ByteBuffer

/**
 * The hand-written idiomatic layer over the generated bindings (ADR 0029's rule: generated
 * code is internal, this is the API). Blocking calls run on [dispatcher]; long work is a job
 * stepped in a loop that checks the coroutine between steps, and cancelling the coroutine
 * cancels the Rust job through its token, so a step already running stops at its next check.
 */
class LaminaDocument private constructor(
    val raw: FfiDocument,
    private val dispatcher: CoroutineDispatcher,
) : AutoCloseable {
    companion object {
        /** Opens a document; the bytes are copied once into Rust, which keeps them. */
        suspend fun open(
            bytes: ByteArray,
            password: String? = null,
            dispatcher: CoroutineDispatcher = Dispatchers.IO,
        ): LaminaDocument = withContext(dispatcher) { LaminaDocument(FfiDocument.open(bytes, password), dispatcher) }

        /**
         * Opens from a direct [ByteBuffer] (UniFFI 0.32's zero-copy `&[u8]`): the buffer is
         * read in place for the call; Rust copies it once to own it.
         */
        suspend fun open(
            direct: ByteBuffer,
            password: String? = null,
            dispatcher: CoroutineDispatcher = Dispatchers.IO,
        ): LaminaDocument =
            withContext(dispatcher) { LaminaDocument(FfiDocument.openBorrowed(direct, password), dispatcher) }
    }

    suspend fun pageCount(): Int = withContext(dispatcher) { raw.pageCount().toInt() }

    suspend fun pages(): List<FfiPageInfo> = withContext(dispatcher) { raw.pageInfos() }

    suspend fun title(): String? = withContext(dispatcher) { raw.info().title }

    suspend fun setTitle(title: String?) = withContext(dispatcher) { raw.setInfo(FfiInfoKey.TITLE, title) }

    /**
     * The output of the newest save that completed, including one that completed although
     * its coroutine was cancelled during the last step: Rust finished the save and rebased
     * the document, so the bytes must not be lost (a cancelled coroutine can't return them).
     */
    @Volatile
    var lastSaved: FfiSaved? = null
        private set

    /**
     * Saves as a job of [unitsPerStep] units per step, reporting progress. Cancelling the
     * calling coroutine cancels the Rust job: the step in flight returns `cancelled` at its
     * next check, and this throws [CancellationException]. If that step had already finished
     * the save, its output is in [lastSaved].
     */
    suspend fun save(
        mode: FfiSaveMode = FfiSaveMode.AUTO,
        unitsPerStep: UInt = 256u,
        onProgress: (FfiSaveStep.Progress) -> Unit = {},
    ): FfiSaved {
        val job = withContext(dispatcher) { raw.saveJob(FfiSaveOptions(mode, false)) }
        job.use {
            while (true) {
                val step =
                    cancellableStep(
                        dispatcher,
                        onCancel = job::cancel,
                        onResultAfterCancel = { if (it is FfiSaveStep.Done) lastSaved = it.saved },
                    ) { job.step(unitsPerStep) }
                when (step) {
                    is FfiSaveStep.Done -> return step.saved.also { lastSaved = it }
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

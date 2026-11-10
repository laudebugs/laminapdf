//! The engine's router against a main process that answers reads slowly: an open that needs
//! many reads and a tile queued behind it both complete. A router that parsed on its own thread
//! would wait forever for bytes only it could deliver.
#![allow(clippy::unwrap_used, clippy::panic, reason = "tests: a panic is a test failure")]

use std::io::{self, PipeReader, PipeWriter};
use std::thread;
use std::time::Duration;

use laminapdf_desktop_protocol::{
    DocId, EngineLimits, FailureKind, FromEngine, MAX_FRAME, PROTOCOL_VERSION, Priority, ReqId, TileFormat,
    TileRequest, ToEngine, read_frame, write_frame,
};
use laminapdf_test_fixtures::{fixture, read_at};

/// The main process's ends of the engine's pipes, and the engine's thread.
fn start_engine() -> (PipeWriter, PipeReader, thread::JoinHandle<()>) {
    let (to_engine_r, to_engine_w) = io::pipe().unwrap();
    let (from_engine_r, from_engine_w) = io::pipe().unwrap();
    let engine = thread::spawn(move || {
        // No renderer: tiles answer RenderUnavailable, which is enough to see them answered.
        laminapdf_desktop_engine::run(to_engine_r, from_engine_w, None).unwrap();
    });
    (to_engine_w, from_engine_r, engine)
}

fn send(w: &mut PipeWriter, msg: &ToEngine, payload: &[u8]) {
    write_frame(w, msg, payload).unwrap();
}

#[test]
fn an_open_waiting_on_slow_reads_and_a_queued_tile_should_both_complete() {
    let bytes = fixture("corpus/fpdf2.pdf");
    let (mut to, mut from, engine) = start_engine();
    send(
        &mut to,
        &ToEngine::Hello {
            version: PROTOCOL_VERSION,
            limits: Box::new(EngineLimits::default()),
        },
        &[],
    );
    let doc = DocId(1);
    send(
        &mut to,
        &ToEngine::OpenDocument {
            req: ReqId(1),
            doc,
            len: bytes.len() as u64,
        },
        &[],
    );
    send(
        &mut to,
        &ToEngine::RenderTile(TileRequest {
            req: ReqId(2),
            doc,
            page: 0,
            scale_milli: 1000,
            x: 0,
            y: 0,
            w: 64,
            h: 64,
            format: TileFormat::Rgba8,
            priority: Priority::Visible,
        }),
        &[],
    );

    let (mut opened, mut tiled, mut reads) = (false, false, 0);
    while !(opened && tiled) {
        let (msg, _) = read_frame::<_, FromEngine>(&mut from, MAX_FRAME).unwrap();
        match msg {
            FromEngine::ReadRange { req, offset, len, .. } => {
                reads += 1;
                thread::sleep(Duration::from_millis(20));
                let mut buf = vec![0; len as usize];
                let n = read_at(&bytes, offset, &mut buf);
                buf.truncate(n);
                send(&mut to, &ToEngine::RangeData { req, offset }, &buf);
            }
            FromEngine::Opened { req: ReqId(1), .. } => opened = true,
            FromEngine::Error {
                req: Some(ReqId(2)),
                kind: FailureKind::RenderUnavailable,
                ..
            } => tiled = true,
            FromEngine::Heartbeat | FromEngine::Ready { .. } => {}
            other => panic!("unexpected {other:?}"),
        }
    }
    send(&mut to, &ToEngine::Shutdown, &[]);
    engine.join().unwrap();
    assert!(reads > 0);
}

#[test]
fn a_cancelled_queued_tile_should_be_answered_cancelled() {
    let (mut to, mut from, engine) = start_engine();
    send(
        &mut to,
        &ToEngine::Hello {
            version: PROTOCOL_VERSION,
            limits: Box::new(EngineLimits::default()),
        },
        &[],
    );
    // The open waits on a read nobody answers, so the tile behind it stays queued.
    send(
        &mut to,
        &ToEngine::OpenDocument {
            req: ReqId(1),
            doc: DocId(1),
            len: 1000,
        },
        &[],
    );
    let tile = TileRequest {
        req: ReqId(2),
        doc: DocId(1),
        page: 0,
        scale_milli: 1000,
        x: 0,
        y: 0,
        w: 8,
        h: 8,
        format: TileFormat::Rgba8,
        priority: Priority::Visible,
    };
    send(&mut to, &ToEngine::RenderTile(tile), &[]);
    // Wait until the worker is blocked on the open's read.
    loop {
        let (msg, _) = read_frame::<_, FromEngine>(&mut from, MAX_FRAME).unwrap();
        if matches!(msg, FromEngine::ReadRange { .. }) {
            break;
        }
    }
    send(&mut to, &ToEngine::Cancel { req: ReqId(2) }, &[]);
    let cancelled = loop {
        let (msg, _) = read_frame::<_, FromEngine>(&mut from, MAX_FRAME).unwrap();
        if let FromEngine::Error {
            req: Some(ReqId(2)),
            kind,
            ..
        } = msg
        {
            break kind;
        }
    };
    drop(to);
    engine.join().unwrap();
    assert_eq!(cancelled, FailureKind::Cancelled);
}

#[test]
fn a_first_message_other_than_hello_should_stop_the_engine() {
    let (to_engine_r, mut to_engine_w) = io::pipe().unwrap();
    let (_from_engine_r, from_engine_w) = io::pipe().unwrap();
    send(&mut to_engine_w, &ToEngine::Shutdown, &[]);
    let result = laminapdf_desktop_engine::run(to_engine_r, from_engine_w, None);
    assert!(result.is_err());
}

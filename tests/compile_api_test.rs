use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use base64::{engine::general_purpose::STANDARD, Engine};
use swissarmynes::server::{self, api::compile_source};
use tower::ServiceExt;

/// Exercise the route shipped to the editor rather than the legacy code generator.
#[tokio::test]
async fn minimal_program_compiles_via_http() {
    let response = server::app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/compile")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"source":"SUB Main()\nEND SUB"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let rom = STANDARD.decode(json["rom"].as_str().unwrap()).unwrap();
    assert_eq!(&rom[..8], &[0x4e, 0x45, 0x53, 0x1a, 8, 1, 0x10, 0]);
    assert_eq!(rom.len(), 139280);
    assert!(json.get("map").is_some());
}

#[test]
fn production_runtime_helpers_are_linked() {
    for source in [
        "DIM w AS WORD\nSUB Main()\n LET w = 1000 + 500\nEND SUB",
        "SUB Main()\n Controller.Read()\nEND SUB",
        "SUB Main()\n Text.Print(10, 10, \"Hello\")\nEND SUB",
    ] {
        compile_source(Some(source.into()), None, None)
            .unwrap_or_else(|error| panic!("{source}\n{error}"));
    }
}

/// Runtime assertions catch bad pointers and reset vectors that textual assembly tests miss.
#[test]
fn compiled_word_math_executes() {
    use tetanes_core::{
        control_deck::{Config, ControlDeck},
        mem::RamState,
    };
    let (rom, _) = compile_source(
        Some("DIM w AS WORD\nSUB Main()\n LET w = 1000 + 500\nEND SUB".into()),
        None,
        None,
    )
    .unwrap();
    let mut deck = ControlDeck::with_config(Config {
        ram_state: RamState::AllZeros,
        ..Config::default()
    });
    deck.load_rom("word.nes", &mut rom.as_slice()).unwrap();
    for _ in 0..10 {
        deck.clock_frame().unwrap();
    }
    assert_eq!(&deck.wram()[0x5c0..0x5c2], &[0xdc, 0x05]);
}

use swissarmynes::server::api::compile_source;
use tetanes_core::{
    control_deck::{Config, ControlDeck},
    mem::RamState,
};

fn run_rom(source: &str) -> ControlDeck {
    let (rom, _) = compile_source(Some(source.to_string()), None, None)
        .unwrap_or_else(|e| panic!("Compile failed for:\n{}\nError: {}", source, e));
    let mut deck = ControlDeck::with_config(Config {
        ram_state: RamState::AllZeros,
        ..Config::default()
    });
    deck.load_rom("test.nes", &mut rom.as_slice()).unwrap();
    for _ in 0..15 {
        deck.clock_frame().unwrap();
    }
    deck
}

#[test]
fn test_sub_parameters_compilation_and_execution() {
    let source = r#"
DIM res AS BYTE

SUB Add(a AS BYTE, b AS BYTE)
    res = a + b
END SUB

SUB Main()
    Add(7, 35)
END SUB
"#;
    let deck = run_rom(source);
    // res at $05C0 should equal 42 (0x2A)
    assert_eq!(deck.wram()[0x5c0], 42);
}

#[test]
fn test_for_loop_downward_step_to_zero_executes() {
    let source = r#"
DIM count AS BYTE
DIM i AS BYTE

SUB Main()
    count = 0
    FOR i = 10 TO 0 STEP -1
        count = count + 1
    NEXT i
END SUB
"#;
    let deck = run_rom(source);
    // count at $05C0 should equal 11 (0x0B)
    assert_eq!(deck.wram()[0x5c0], 11);
}

#[test]
fn test_select_case_ranges_and_is_compilation_and_execution() {
    let source = r#"
DIM val AS BYTE
DIM res1 AS BYTE
DIM res2 AS BYTE
DIM res3 AS BYTE

SUB Main()
    val = 5
    SELECT CASE val
        CASE 1 TO 10
            res1 = 100
        CASE ELSE
            res1 = 1
    END SELECT

    val = 25
    SELECT CASE val
        CASE IS > 20
            res2 = 200
        CASE ELSE
            res2 = 2
    END SELECT

    val = 15
    SELECT CASE val
        CASE 1 TO 10
            res3 = 100
        CASE IS > 20
            res3 = 200
        CASE ELSE
            res3 = 50
    END SELECT
END SUB
"#;
    let deck = run_rom(source);
    // res1 @ $05C1 = 100
    // res2 @ $05C2 = 200
    // res3 @ $05C3 = 50
    assert_eq!(deck.wram()[0x5c1], 100);
    assert_eq!(deck.wram()[0x5c2], 200);
    assert_eq!(deck.wram()[0x5c3], 50);
}

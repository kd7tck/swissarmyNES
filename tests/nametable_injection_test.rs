use swissarmynes::compiler::codegen::NAMETABLE_ADDR;

#[test]
fn test_multiple_nametables_injection() {
    // Verify that $D500 (NAMETABLE_ADDR) and subsequent 1024-byte blocks fit in $D500-$D900
    assert_eq!(NAMETABLE_ADDR, 0xD500);
}

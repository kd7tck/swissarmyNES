# nestest fixture provenance

Source repository: https://github.com/christopherpow/nes-test-roms
Pinned revision: 95d8f621ae55cee0d09b91519a8989ae0e64753b
Upstream paths: other/nestest.nes, other/nestest.log, other/nestest.txt
ROM author: Kevin Horton.
Online NES Test README reference: https://raw.githubusercontent.com/christopherpow/nes-test-roms/95d8f621ae55cee0d09b91519a8989ae0e64753b/other/nestest.txt (browse online at https://github.com/christopherpow/nes-test-roms/blob/master/other/nestest.txt)

Run `node scripts/fetch-test-fixtures.mjs` to obtain these external test inputs locally (`nestest.nes`, `nestest.trace`, and `nestest-readme.txt`). Files are verified and preserved byte-for-byte. Compiled ROMs and downloaded fixture payloads are excluded from version control per payload policy, but are downloaded automatically into `emulator/tests/fixtures/` for local development.

SHA-256:
- nestest.nes: f67d55fd6b3cf0bad1cc85f1df0d739c65b53e79cecb7fea8f77ec0eadab0004
- nestest.trace: 627c8e180b1a924dfa705c5dc6958fad7ab75a62de556173caf880ccc1337540

The reference contains 8991 pre-instruction records. Initial state: PC=C000, A=X=Y=00, P=24, SP=FD, CPU cycles=7. Last reference record: PC=C66E, A=00, X=FF, Y=15, P=27, SP=FD, cycles=26554. The test modifies only the reset vector in an in-memory ROM copy to select the documented C000 automation entry. Downloaded fixture bytes remain unchanged.

Current result: all 8991 records match PC/A/X/Y/P/SP/cycles exactly. The production snapshot publishes the conventional U=1/B=0 representation; these two stack serialization bits are not physical CPU flags (https://www.nesdev.org/wiki/Status_flag). All six physical flags remain unchanged. Separate exhaustive tests verify PLP/PHP and RTI with every possible stack-status byte. The comparator rejects changes to each field and every status bit, including B/U.

The successful automation path does not initialize or write $02/$03; these bytes are verified unchanged from production random power-on RAM rather than incorrectly required to start at zero. Supplemental CPU/interrupt/DMA coverage and browser equivalence remain pending.

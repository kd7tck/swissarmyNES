// Download external test inputs without checking compiled ROMs into source control.
import {createHash} from 'node:crypto';
import {mkdir, readFile, writeFile} from 'node:fs/promises';

const revision = '95d8f621ae55cee0d09b91519a8989ae0e64753b';
const fixtures = [
    ['nestest.nes', 'nestest.nes', 'f67d55fd6b3cf0bad1cc85f1df0d739c65b53e79cecb7fea8f77ec0eadab0004'],
    ['nestest.log', 'nestest.trace', '627c8e180b1a924dfa705c5dc6958fad7ab75a62de556173caf880ccc1337540'],
    ['nestest.txt', 'nestest-readme.txt', '3792dcbba90dc7e16a7a46fe7559aedd942519bf61e44fad806aaa1070366712'],
];
const directory = new URL('../emulator/tests/fixtures/', import.meta.url);
const digest = bytes => createHash('sha256').update(bytes).digest('hex');
await mkdir(directory, {recursive: true});
for (const [upstream, local, checksum] of fixtures) {
    const destination = new URL(local, directory);
    let bytes;
    try { bytes = await readFile(destination); }
    catch (error) { if (error.code !== 'ENOENT') throw error; }
    if (bytes) {
        if (digest(bytes) !== checksum) throw new Error(`${local}: existing file checksum mismatch`);
        console.log(`${local}: verified`);
        continue;
    }
    const response = await fetch(`https://raw.githubusercontent.com/christopherpow/nes-test-roms/${revision}/other/${upstream}`);
    if (!response.ok) throw new Error(`${local}: download failed (${response.status})`);
    bytes = Buffer.from(await response.arrayBuffer());
    if (digest(bytes) !== checksum) throw new Error(`${local}: downloaded checksum mismatch`);
    // Exclusive creation preserves any file another process created during download.
    await writeFile(destination, bytes, {flag: 'wx'});
    console.log(`${local}: downloaded and verified`);
}

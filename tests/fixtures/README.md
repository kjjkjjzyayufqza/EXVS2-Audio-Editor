# song_wgnmd1-tone.bin

The 308-byte TONE section from the reported `song_wgnmd1.nus3bank` file
(original range `0x64c..0x780`, exclusive end). No audio payload is included.

Its normal, unprefixed header names `song_wgnmd1` and references 3,439,408
bytes at PACK offset 0. Its parameter tail is not compatible with the
editor's full EXVS2 metadata parser. The old parser instead accepted an
8-byte-shifted interpretation: a name containing 78 NUL bytes and size 6.

The regression tests substitute a tiny PACK payload and patch only the
header's payload-size field. They verify the name and payload, metadata
preservation across save/reopen, range validation, and actual WAV decoding.

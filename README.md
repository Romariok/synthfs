# SynthFS - A Musical File System Based on NFSv3

## Overview
SynthFS is a unique implementation of a musical file system built on top of the Rust NFSv3 server. It allows you to create and play music by manipulating files in a special directory structure.

## Installation

1. Clone the NFSv3 server implementation:
   ```bash
   git clone https://github.com/xetdata/nfsserve
   ```

2. Copy `synthfs.rs` to the `examples` directory of the project and `/sounds` directory to the root of the project

3. Build the project:
   ```bash
   cargo build --example synthfs --features synthfs
   ```

4. Run the server:
   ```bash
   ./target/debug/examples/synthfs
   ```

5. Mount the filesystem:
   ```bash
   mount.nfs -o user,noacl,nolock,vers=3,tcp,wsize=1048576,rsize=131072,actimeo=120,port=11111,mountport=11111 localhost:/ synthfs
   ```

## Creating Music

### File Naming Convention
To create a musical note, create a file with the following format:

```
<sequence_number>_<instrument_type>_<note>.txt
```

### Available Instruments
- lancer
- bell
- organ
- sine

### Available Notes

- A, A#, A2, A#2
- B, B2
- C, C#, C2, C#2
- D, D#, D2, D#2
- E, E2
- F, F#, F2, F#2
- G, G#, G2, G#2

### Playing Music
To play the sequence of notes in a directory, simply use:

```bash
cat *
```

## Notes
- Each directory acts as a separate musical composition
- Files must follow the exact naming convention to be recognized
- Notes are played in the order of their sequence numbers
- Multiple instruments can be used in the same sequence
use std::sync::mpsc;
use std::sync::Mutex;
use std::time::SystemTime;
use std::thread;

use rodio::{OutputStream, OutputStreamHandle};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use tracing::debug;


use async_trait::async_trait;

use nfsserve::{
    nfs::{
        self, fattr3, fileid3, filename3, ftype3, nfspath3, nfsstat3, nfstime3, sattr3, specdata3,
    },
    tcp::*,
    vfs::{DirEntry, NFSFileSystem, ReadDirResult, VFSCapabilities},
};

const SOUNDS: &[&str] = &[
    "A", "A#", "A2", "A#2", "B", "B2", "C", "C#", "C2", "C#2", "D", "D#", "D2", "D#2", "E", "E2",
    "F", "F#", "F2", "F#2", "G", "G#", "G2", "G#2",
];

const SOUNDS_TYPES: &[&str] = &["bell", "lancer", "organ", "sine"];

pub struct SynthFS {
    fs: Mutex<Vec<FSEntry>>,
    rootdir: fileid3,
    sound_sender: mpsc::Sender<(String, String)>,
}

struct AudioPlayer {
    _stream: OutputStream,
    handle: OutputStreamHandle,
}

impl AudioPlayer {
    fn new() -> Self {
        let (stream, handle) = OutputStream::try_default().unwrap();
        Self {
            _stream: stream,
            handle,
        }
    }

    fn play_sound(&self, keycode: String, sound_type: String) {
        let file_name = keycode.clone() + ".flac";
        let file_path = format!("sounds/{}/{}", sound_type, file_name);
        debug!("Playing sound: {}", file_path);
        match File::open(Path::new(&file_path)) {
            Ok(file) => {
                if let Ok(source) = rodio::Decoder::new(BufReader::new(file)) {
                    if let Ok(sink) = rodio::Sink::try_new(&self.handle) {
                        sink.append(source);
                        sink.sleep_until_end();
                    } else {
                        debug!("Error creating sink");
                    }
                } else {
                    debug!("Error creating source");
                }
            }
            Err(e) => {
                debug!("Error opening sound file {}: {}", file_path, e);
            }
        }
    }
}

impl std::fmt::Debug for SynthFS {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SynthFS")
            .field("fs", &self.fs)
            .field("rootdir", &self.rootdir)
            .finish_non_exhaustive()
    }
}

impl Default for SynthFS {
    fn default() -> SynthFS {
        let entries = vec![
            make_file("", 0, 0, &[]), // fileid 0 is special
            make_dir(
                "/",
                1,           // current id. Must match position in entries
                1,           // parent id
                vec![2, 28], // children
            ),
            make_dir(
                "sine",
                2,
                1,
                vec![
                    3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
                    24, 25, 26, 27,
                ],
            ),
            make_file("1_sine_A.txt", 3, 2, "A".as_bytes()),
            make_file("2_sine_A#.txt", 4, 2, "A#".as_bytes()),
            make_file("3_sine_A2.txt", 5, 2, "A2".as_bytes()),
            make_file("4_sine_A#2.txt", 6, 2, "A#2".as_bytes()),
            make_file("5_bell_B.txt", 7, 2, "B".as_bytes()),
            make_file("6_bell_B2.txt", 8, 2, "B2".as_bytes()),
            make_file("7_bell_C.txt", 9, 2, "C".as_bytes()),
            make_file("8_bell_C#.txt", 10, 2, "C#".as_bytes()),
            make_file("9_bell_C2.txt", 11, 2, "C2".as_bytes()),
            make_file("10_bell_C#2.txt", 12, 2, "C#2".as_bytes()),
            make_file("11_bell_D.txt", 13, 2, "D".as_bytes()),
            make_file("12_bell_D#.txt", 14, 2, "D#".as_bytes()),
            make_file("13_organ_D2.txt", 15, 2, "D2".as_bytes()),
            make_file("14_organ_D#2.txt", 16, 2, "D#2".as_bytes()),
            make_file("15_organ_E.txt", 17, 2, "E".as_bytes()),
            make_file("16_organ_E2.txt", 18, 2, "E2".as_bytes()),
            make_file("17_lancer_F.txt", 19, 2, "F".as_bytes()),
            make_file("18_lancer_F#.txt", 20, 2, "F#".as_bytes()),
            make_file("19_lancer_F2.txt", 21, 2, "F2".as_bytes()),
            make_file("20_lancer_F#2.txt", 22, 2, "F#2".as_bytes()),
            make_file("21_lancer_G.txt", 23, 2, "G".as_bytes()),
            make_file("22_lancer_G#.txt", 24, 2, "G#".as_bytes()),
            make_file("23_lancer_G2.txt", 25, 2, "G2".as_bytes()),
            make_file("24_lancer_G#2.txt", 26, 2, "G#2".as_bytes()),
            make_file("play.txt", 27, 2, "PLAY".as_bytes()),
            make_dir("song1", 28, 1, vec![29, 30, 31, 32, 33, 34, 35]),
            make_file("1_lancer_C.txt", 29, 28, "C".as_bytes()),
            make_file("2_lancer_D.txt", 30, 28, "D".as_bytes()),
            make_file("3_lancer_E.txt", 31, 28, "E".as_bytes()),
            make_file("4_lancer_C.txt", 32, 28, "C".as_bytes()),
            make_file("5_lancer_D.txt", 33, 28, "D".as_bytes()),
            make_file("6_lancer_C.txt", 34, 28, "C".as_bytes()),
            make_file("play.txt", 35, 28, "PLAY".as_bytes()),
        ];

        let (sender, receiver) = mpsc::channel();

        // Запускаем отдельный поток для воспроизведения звука
        std::thread::spawn(move || {
            let player = AudioPlayer::new();
            while let Ok((keycode, sound_type)) = receiver.recv() {
                player.play_sound(keycode, sound_type);
            }
        });

        SynthFS {
            fs: Mutex::new(entries),
            rootdir: 1,
            sound_sender: sender,
        }
    }
}

#[derive(Debug, Clone)]
enum FSContents {
    File(Vec<u8>),
    Directory(Vec<fileid3>),
}
#[allow(dead_code)]
#[derive(Debug, Clone)]
struct FSEntry {
    id: fileid3,
    attr: fattr3,
    name: filename3,
    parent: fileid3,
    contents: FSContents,
}

fn make_file(name: &str, id: fileid3, parent: fileid3, contents: &[u8]) -> FSEntry {
    let attr = fattr3 {
        ftype: ftype3::NF3REG,
        mode: 0o755,
        nlink: 1,
        uid: 507,
        gid: 507,
        size: contents.len() as u64,
        used: contents.len() as u64,
        rdev: specdata3::default(),
        fsid: 0,
        fileid: id,
        atime: nfstime3::default(),
        mtime: nfstime3::default(),
        ctime: nfstime3::default(),
    };
    FSEntry {
        id,
        attr,
        name: name.as_bytes().into(),
        parent,
        contents: FSContents::File(contents.to_vec()),
    }
}

fn make_dir(name: &str, id: fileid3, parent: fileid3, contents: Vec<fileid3>) -> FSEntry {
    let attr = fattr3 {
        ftype: ftype3::NF3DIR,
        mode: 0o777,
        nlink: 1,
        uid: 507,
        gid: 507,
        size: 0,
        used: 0,
        rdev: specdata3::default(),
        fsid: 0,
        fileid: id,
        atime: nfstime3::default(),
        mtime: nfstime3::default(),
        ctime: nfstime3::default(),
    };
    FSEntry {
        id,
        attr,
        name: name.as_bytes().into(),
        parent,
        contents: FSContents::Directory(contents),
    }
}

#[async_trait]
impl NFSFileSystem for SynthFS {
    fn root_dir(&self) -> fileid3 {
        self.rootdir
    }

    fn capabilities(&self) -> VFSCapabilities {
        VFSCapabilities::ReadWrite
    }

    async fn write(&self, id: fileid3, offset: u64, data: &[u8]) -> Result<fattr3, nfsstat3> {
        {
            let mut fs = self.fs.lock().unwrap();
            let mut fssize = fs[id as usize].attr.size;
            if let FSContents::File(bytes) = &mut fs[id as usize].contents {
                let offset = offset as usize;
                if offset + data.len() > bytes.len() {
                    bytes.resize(offset + data.len(), 0);
                    bytes[offset..].copy_from_slice(data);
                    fssize = bytes.len() as u64;
                }
            }
            fs[id as usize].attr.size = fssize;
            fs[id as usize].attr.used = fssize;
        }
        self.getattr(id).await
    }

    async fn create(
        &self,
        dirid: fileid3,
        filename: &filename3,
        _attr: sattr3,
    ) -> Result<(fileid3, fattr3), nfsstat3> {
        let newid: fileid3;
        {
            let mut fs = self.fs.lock().unwrap();
            newid = fs.len() as fileid3;
            fs.push(make_file(
                std::str::from_utf8(filename).unwrap(),
                newid,
                dirid,
                "".as_bytes(),
            ));
            if let FSContents::Directory(dir) = &mut fs[dirid as usize].contents {
                dir.push(newid);
            }
        }
        Ok((newid, self.getattr(newid).await.unwrap()))
    }

    async fn create_exclusive(
        &self,
        _dirid: fileid3,
        _filename: &filename3,
    ) -> Result<fileid3, nfsstat3> {
        Err(nfsstat3::NFS3ERR_NOTSUPP)
    }

    async fn lookup(&self, dirid: fileid3, filename: &filename3) -> Result<fileid3, nfsstat3> {
        let fs = self.fs.lock().unwrap();
        let entry = fs.get(dirid as usize).ok_or(nfsstat3::NFS3ERR_NOENT)?;
        if let FSContents::File(_) = entry.contents {
            return Err(nfsstat3::NFS3ERR_NOTDIR);
        } else if let FSContents::Directory(dir) = &entry.contents {
            // if looking for dir/. its the current directory
            if filename[..] == [b'.'] {
                return Ok(dirid);
            }
            // if looking for dir/.. its the parent directory
            if filename[..] == [b'.', b'.'] {
                return Ok(entry.parent);
            }
            for i in dir {
                if let Some(f) = fs.get(*i as usize) {
                    if f.name[..] == filename[..] {
                        return Ok(*i);
                    }
                }
            }
        }
        Err(nfsstat3::NFS3ERR_NOENT)
    }
    async fn getattr(&self, id: fileid3) -> Result<fattr3, nfsstat3> {
        let fs = self.fs.lock().unwrap();
        let entry = fs.get(id as usize).ok_or(nfsstat3::NFS3ERR_NOENT)?;
        Ok(entry.attr)
    }
    async fn setattr(&self, id: fileid3, setattr: sattr3) -> Result<fattr3, nfsstat3> {
        let mut fs = self.fs.lock().unwrap();
        let entry = fs.get_mut(id as usize).ok_or(nfsstat3::NFS3ERR_NOENT)?;
        match setattr.atime {
            nfs::set_atime::DONT_CHANGE => {}
            nfs::set_atime::SET_TO_CLIENT_TIME(c) => {
                entry.attr.atime = c;
            }
            nfs::set_atime::SET_TO_SERVER_TIME => {
                let d = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap();
                entry.attr.atime.seconds = d.as_secs() as u32;
                entry.attr.atime.nseconds = d.subsec_nanos();
            }
        };
        match setattr.mtime {
            nfs::set_mtime::DONT_CHANGE => {}
            nfs::set_mtime::SET_TO_CLIENT_TIME(c) => {
                entry.attr.mtime = c;
            }
            nfs::set_mtime::SET_TO_SERVER_TIME => {
                let d = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap();
                entry.attr.mtime.seconds = d.as_secs() as u32;
                entry.attr.mtime.nseconds = d.subsec_nanos();
            }
        };
        match setattr.uid {
            nfs::set_uid3::uid(u) => {
                entry.attr.uid = u;
            }
            nfs::set_uid3::Void => {}
        }
        match setattr.gid {
            nfs::set_gid3::gid(u) => {
                entry.attr.gid = u;
            }
            nfs::set_gid3::Void => {}
        }
        match setattr.size {
            nfs::set_size3::size(s) => {
                entry.attr.size = s;
                entry.attr.used = s;
                if let FSContents::File(bytes) = &mut entry.contents {
                    bytes.resize(s as usize, 0);
                }
            }
            nfs::set_size3::Void => {}
        }
        Ok(entry.attr)
    }

    async fn read(
        &self,
        id: fileid3,
        offset: u64,
        count: u32,
    ) -> Result<(Vec<u8>, bool), nfsstat3> {
        let fs = self.fs.lock().unwrap();
        let entry = fs.get(id as usize).ok_or(nfsstat3::NFS3ERR_NOENT)?;
        if let FSContents::Directory(_) = entry.contents {
            return Err(nfsstat3::NFS3ERR_ISDIR);
        } else if let FSContents::File(bytes) = &entry.contents {
            let mut start = offset as usize;
            let mut end = offset as usize + count as usize;
            let eof = end >= bytes.len();
            if start >= bytes.len() {
                start = bytes.len();
            }
            if end > bytes.len() {
                end = bytes.len();
            }
            let mut name = String::from_utf8_lossy(&entry.name).to_string();
            if name.starts_with("play") {
                if let FSContents::Directory(dir) = &fs[entry.parent as usize].contents {
                    let mut sound_names = Vec::new();
                    for &id in dir {
                        if let Some(entry) = fs.get(id as usize) {
                            name = String::from_utf8_lossy(&entry.name).to_string();
                            if name.ends_with(".txt") && !name.starts_with("play") {
                                let full_name = name[..name.len()-4].to_string();
                                if let (Some(sound_type), Some(note_name)) = (
                                    full_name.split('_').nth(1).map(|s| s.to_lowercase()),
                                    full_name.split('_').nth(2).map(|s| s.to_uppercase())
                                ) {
                                    if SOUNDS_TYPES.contains(&sound_type.as_str()) {
                                        if SOUNDS.contains(&note_name.as_str()) {
                                            sound_names.push(full_name);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    sound_names.sort();
                    for name in sound_names {
                        let note = name.split('_').nth(2).unwrap().to_uppercase();
                        let sound_type = name.split('_').nth(1).unwrap().to_lowercase();
                        if let Err(e) = self.sound_sender.send((note, sound_type)) {
                            debug!("Error sending sound command: {}", e);
                        }
                        thread::sleep(std::time::Duration::from_millis(300));
                    }
                }
            } else if name.ends_with(".txt") {
                if let Some(sound_type) = name.split('_').nth(1) {
                    if let Some(note) = name.split('_').nth(2) {
                        let note = note[..note.len()-4].to_uppercase();
                        let sound_type = sound_type.to_lowercase();
                        if SOUNDS.contains(&note.as_str()) && SOUNDS_TYPES.contains(&sound_type.as_str()) {
                            if let Err(e) = self.sound_sender.send((note, sound_type)) {
                                debug!("Error sending sound command: {}", e);
                            }
                        }
                    }
                }
            }

            return Ok((bytes[start..end].to_vec(), eof));
        }
        Err(nfsstat3::NFS3ERR_NOENT)
    }

    async fn readdir(
        &self,
        dirid: fileid3,
        start_after: fileid3,
        max_entries: usize,
    ) -> Result<ReadDirResult, nfsstat3> {
        let fs = self.fs.lock().unwrap();
        let entry = fs.get(dirid as usize).ok_or(nfsstat3::NFS3ERR_NOENT)?;
        if let FSContents::File(_) = entry.contents {
            return Err(nfsstat3::NFS3ERR_NOTDIR);
        } else if let FSContents::Directory(dir) = &entry.contents {
            let mut ret = ReadDirResult {
                entries: Vec::new(),
                end: false,
            };
            let mut start_index = 0;
            if start_after > 0 {
                if let Some(pos) = dir.iter().position(|&r| r == start_after) {
                    start_index = pos + 1;
                } else {
                    return Err(nfsstat3::NFS3ERR_BAD_COOKIE);
                }
            }
            let remaining_length = dir.len() - start_index;

            for i in dir[start_index..].iter() {
                ret.entries.push(DirEntry {
                    fileid: *i,
                    name: fs[(*i) as usize].name.clone(),
                    attr: fs[(*i) as usize].attr,
                });
                if ret.entries.len() >= max_entries {
                    break;
                }
            }
            if ret.entries.len() == remaining_length {
                ret.end = true;
            }
            return Ok(ret);
        }
        Err(nfsstat3::NFS3ERR_NOENT)
    }

    async fn remove(&self, dirid: fileid3, filename: &filename3) -> Result<(), nfsstat3> {
        let mut fs = self.fs.lock().unwrap();

        let mut file_id = None;

        if let FSContents::Directory(dir) = &fs[dirid as usize].contents {
            for &id in dir {
                if fs[id as usize].name[..] == filename[..] {
                    file_id = Some(id);
                    break;
                }
            }
        }

        if let Some(id) = file_id {
            if let FSContents::Directory(dir) = &mut fs[dirid as usize].contents {
                dir.retain(|&x| x != id);
            }
        } else {
            return Err(nfsstat3::NFS3ERR_NOENT);
        }
        Ok(())
    }

    async fn rename(
        &self,
        from_dirid: fileid3,
        from_filename: &filename3,
        to_dirid: fileid3,
        to_filename: &filename3,
    ) -> Result<(), nfsstat3> {
        let mut fs = self.fs.lock().unwrap();

        let mut file_id = None;
        let mut file_bytes = None;

        if let FSContents::Directory(dir) = &fs[from_dirid as usize].contents {
            for &id in dir {
                if fs[id as usize].name[..] == from_filename[..] {
                    file_id = Some(id);
                    if let FSContents::File(bytes) = &fs[id as usize].contents {
                        file_bytes = Some(bytes.clone());
                    }
                    break;
                }
            }
        }

        if let (Some(id), Some(bytes)) = (file_id, file_bytes) {
            if let FSContents::Directory(dir) = &mut fs[from_dirid as usize].contents {
                dir.retain(|&x| x != id);
            }
            
            fs[id as usize] = make_file(
                std::str::from_utf8(to_filename).unwrap(),
                id,
                to_dirid,
                &bytes,
            );
            
            if let FSContents::Directory(dir) = &mut fs[to_dirid as usize].contents {
                dir.push(id);
            }
            Ok(())
        } else {
            Err(nfsstat3::NFS3ERR_NOENT)
        }
    }

    async fn mkdir(
        &self,
        dirid: fileid3,
        dirname: &filename3,
    ) -> Result<(fileid3, fattr3), nfsstat3> {
        let newid: fileid3;
        {
            let mut fs = self.fs.lock().unwrap();
            newid = fs.len() as fileid3;
            fs.push(make_dir(
                std::str::from_utf8(dirname).unwrap(),
                newid,
                dirid,
                vec![],
            ));
            if let FSContents::Directory(dir) = &mut fs[dirid as usize].contents {
                dir.push(newid);
            }
        }
        Ok((newid, self.getattr(newid).await.unwrap()))
    }

    async fn symlink(
        &self,
        _dirid: fileid3,
        _linkname: &filename3,
        _symlink: &nfspath3,
        _attr: &sattr3,
    ) -> Result<(fileid3, fattr3), nfsstat3> {
        Err(nfsstat3::NFS3ERR_ROFS)
    }
    async fn readlink(&self, _id: fileid3) -> Result<nfspath3, nfsstat3> {
        return Err(nfsstat3::NFS3ERR_NOTSUPP);
    }
}

const HOSTPORT: u32 = 11111;

#[tokio::main]
#[allow(unused_variables)]
async fn main() {
    let init = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_writer(std::io::stderr)
        .init();
    let listener = NFSTcpListener::bind(&format!("127.0.0.1:{HOSTPORT}"), SynthFS::default())
        .await
        .unwrap();

    listener.handle_forever().await.unwrap();
}

use std::{
    error::Error,
    io::{self, Read},
};
use swarm_engine::{
    check_command_input_size, read_command_document, CommandInputFailure, COMMAND_INPUT_MAX_BYTES,
};
#[test]
fn exact_transport_boundary_and_stream_stop_are_not_execution_timeouts() {
    let mut control = io::Cursor::new(b"valid ordinary document");
    assert_eq!(
        read_command_document(&mut control).unwrap(),
        b"valid ordinary document"
    );
    let boundary = vec![b' '; COMMAND_INPUT_MAX_BYTES];
    assert!(check_command_input_size(&boundary).is_ok());
    assert_eq!(
        read_command_document(&mut io::Cursor::new(&boundary)).unwrap(),
        boundary
    );
    let mut beyond = boundary;
    beyond.push(b' ');
    assert_eq!(
        check_command_input_size(&beyond).unwrap_err().failure,
        CommandInputFailure::TooLarge
    );
    struct Endless {
        read: usize,
    }
    impl Read for Endless {
        fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
            b.fill(b'x');
            self.read += b.len();
            Ok(b.len())
        }
    }
    let mut stream = Endless { read: 0 };
    assert_eq!(
        read_command_document(&mut stream).unwrap_err().failure,
        CommandInputFailure::TooLarge
    );
    assert_eq!(stream.read, COMMAND_INPUT_MAX_BYTES + 1);
}
#[test]
fn read_failures_keep_their_original_cause_without_private_echo_and_recover() {
    assert_eq!(
        read_command_document(&mut io::Cursor::new(b"control")).unwrap(),
        b"control"
    );
    struct Denied;
    impl Read for Denied {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "PRIVATE read cause",
            ))
        }
    }
    let error = read_command_document(&mut Denied).unwrap_err();
    assert_eq!(error.failure, CommandInputFailure::Io);
    assert_eq!(
        error
            .source()
            .unwrap()
            .downcast_ref::<io::Error>()
            .unwrap()
            .kind(),
        io::ErrorKind::PermissionDenied
    );
    assert!(!format!("{error:?}").contains("PRIVATE"));
    assert_eq!(
        read_command_document(&mut io::Cursor::new(b"recovered")).unwrap(),
        b"recovered"
    );
}
#[cfg(unix)]
#[test]
fn named_regular_symlink_boundary_sparse_fifo_and_device_contracts() {
    use std::{
        fs,
        os::unix::fs::{symlink, PermissionsExt},
    };
    use swarm_engine::read_command_file;
    let root =
        std::env::temp_dir().join(swarm_engine::generate_identifier("command-input-").unwrap());
    fs::create_dir(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    let file = root.join("request.json");
    fs::write(&file, b"control").unwrap();
    assert_eq!(read_command_file(&file).unwrap(), b"control");
    symlink(&file, root.join("alias.json")).unwrap();
    assert_eq!(
        read_command_file(&root.join("alias.json")).unwrap(),
        b"control"
    );
    let sparse = fs::File::create(root.join("oversize.json")).unwrap();
    sparse
        .set_len((COMMAND_INPUT_MAX_BYTES + 1) as u64)
        .unwrap();
    drop(sparse);
    assert_eq!(
        read_command_file(&root.join("oversize.json"))
            .unwrap_err()
            .failure,
        CommandInputFailure::TooLarge
    );
    assert_eq!(read_command_file(&file).unwrap(), b"control");
    let fifo = root.join("request.fifo");
    let directory = fs::File::open(&root).unwrap();
    rustix::fs::mkfifoat(
        &directory,
        "request.fifo",
        rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR,
    )
    .unwrap();
    drop(directory);
    assert_eq!(
        read_command_file(&fifo).unwrap_err().failure,
        CommandInputFailure::NotRegular
    );
    assert_eq!(read_command_file(&file).unwrap(), b"control");
    assert_eq!(
        read_command_file(std::path::Path::new("/dev/zero"))
            .unwrap_err()
            .failure,
        CommandInputFailure::NotRegular
    );
    assert_eq!(read_command_file(&file).unwrap(), b"control");
    assert_eq!(
        read_command_file(&root).unwrap_err().failure,
        CommandInputFailure::NotRegular
    );
    assert_eq!(read_command_file(&file).unwrap(), b"control");
    fs::remove_dir_all(root).unwrap();
}

use std::process::Command;

#[test]
fn neomacsclient_version_matches_emacs_version() {
    let output = Command::new(env!("CARGO_BIN_EXE_neomacsclient"))
        .arg("--version")
        .output()
        .expect("neomacsclient --version should run");

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "neomacsclient 31.0.50\n"
    );
}

#[cfg(unix)]
#[test]
fn neomacsclient_sends_gnu_server_request_over_local_socket() {
    use std::fs;
    use std::io::{Read, Write};
    use std::os::unix::net::UnixListener;
    use std::path::PathBuf;
    use std::thread;

    let repo_tmp = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("neomacs-bin should live under the repository root")
        .join("tmp");
    fs::create_dir_all(&repo_tmp).expect("repo-local tmp dir");
    let dir = tempfile::Builder::new()
        .prefix("neomacsclient-cli-")
        .tempdir_in(repo_tmp)
        .expect("repo-local tempdir");
    let socket = dir.path().join("server");
    let listener = UnixListener::bind(&socket).expect("bind local socket");

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept client");
        let mut request = Vec::new();
        let mut byte = [0u8; 1];
        loop {
            stream.read_exact(&mut byte).expect("read request byte");
            request.push(byte[0]);
            if byte[0] == b'\n' {
                break;
            }
        }
        stream
            .write_all(b"-print OK&&done\n")
            .expect("write response");
        String::from_utf8(request).expect("utf8 request")
    });

    let output = Command::new(env!("CARGO_BIN_EXE_neomacsclient"))
        .arg("--socket-name")
        .arg(&socket)
        .arg("--no-wait")
        .arg("--eval")
        .arg("(message \"a b\")")
        .output()
        .expect("neomacsclient should run");

    let request = server.join().expect("server thread should finish");
    assert!(output.status.success(), "{output:?}");
    assert_eq!(String::from_utf8_lossy(&output.stdout), "OK&done");
    assert!(request.starts_with("-dir "));
    assert!(request.contains(" -nowait "));
    assert!(request.contains(" -current-frame "));
    assert!(request.contains(" -eval (message&_\"a&_b\") "));
    assert!(request.ends_with(" \n"));
}

#[test]
fn neomacsclient_sends_gnu_auth_for_tcp_server_file() {
    use std::fs;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::path::PathBuf;
    use std::thread;

    let repo_tmp = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("neomacs-bin should live under the repository root")
        .join("tmp");
    fs::create_dir_all(&repo_tmp).expect("repo-local tmp dir");
    let dir = tempfile::Builder::new()
        .prefix("neomacsclient-tcp-")
        .tempdir_in(repo_tmp)
        .expect("repo-local tempdir");
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind tcp listener");
    let port = listener.local_addr().expect("local addr").port();
    let auth_key = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!?";
    let auth_file = dir.path().join("server-auth");
    fs::write(&auth_file, format!("127.0.0.1:{port} 12345\n{auth_key}")).expect("write auth file");

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept client");
        let mut request = Vec::new();
        let mut byte = [0u8; 1];
        loop {
            stream.read_exact(&mut byte).expect("read request byte");
            request.push(byte[0]);
            if byte[0] == b'\n' {
                break;
            }
        }
        stream
            .write_all(b"-print TCP_OK\n")
            .expect("write response");
        String::from_utf8(request).expect("utf8 request")
    });

    let output = Command::new(env!("CARGO_BIN_EXE_neomacsclient"))
        .arg("--server-file")
        .arg(&auth_file)
        .arg("--eval")
        .arg("(+ 1 2)")
        .output()
        .expect("neomacsclient should run");

    let request = server.join().expect("server thread should finish");
    assert!(output.status.success(), "{output:?}");
    assert_eq!(String::from_utf8_lossy(&output.stdout), "TCP_OK");
    assert!(request.starts_with(&format!("-auth {auth_key} -dir ")));
    assert!(request.contains(" -current-frame "));
    assert!(request.contains(" -eval (+&_1&_2) "));
    assert!(request.ends_with(" \n"));
}

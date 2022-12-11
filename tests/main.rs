use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::Path;
use std::thread;
use std::time::Duration;

const DEVICE_PATH: &str = "/dev/chardev";
const MAX_STRING_LENGTH: usize = 4096;
const MAX_MESSAGES: usize = 1000;

// Read up to a newline.
fn read_line(file: &mut File) -> io::Result<String> {
    BufReader::new(file).lines().next().unwrap()
}

// Write a string, appending a newline.
fn write_line(file: &mut File, line: &str) -> io::Result<()> {
    // Append a newline to the line
    let line = format!("{line}\n");
    file.write_all(line.as_bytes())
}

// Do a single read call. Not dependent on a trailing newline.
fn read_str(file: &mut File) -> io::Result<String> {
    let mut buf = [0; MAX_STRING_LENGTH];
    let bytes = file.read(&mut buf)?;
    Ok(String::from_utf8(buf[..bytes].to_vec()).unwrap())
}

// Write a string. Does not append a newline.
fn write_str(file: &mut File, line: &str) -> io::Result<()> {
    file.write_all(line.as_bytes())
}

// Read bytes.
fn read_bytes(file: &mut File) -> io::Result<Vec<u8>> {
    let mut buf = [0; MAX_STRING_LENGTH];
    let bytes = file.read(&mut buf)?;
    Ok(buf[..bytes].to_vec())
}

// Write bytes.
fn write_bytes(file: &mut File, bytes: &[u8]) -> io::Result<()> {
    file.write_all(bytes)
}

// Open the device for read and write.
fn open() -> File {
    OpenOptions::new()
        .read(true)
        .write(true)
        .open(DEVICE_PATH)
        .unwrap()
}

#[test]
fn test_device_exists() {
    assert!(Path::new(DEVICE_PATH).exists());
}

#[test]
fn test_write_read_short() {
    let mut file = open();
    let line = "Hello, World!";
    write_line(&mut file, line).unwrap();
    assert_eq!(read_line(&mut file).unwrap(), line);
}

#[test]
fn test_write_read_short_no_newline() {
    let mut file = open();
    let line = "Hello, World!";
    write_str(&mut file, line).unwrap();
    assert_eq!(read_str(&mut file).unwrap(), line);
}

#[test]
fn test_write_read_short_null_byte() {
    let mut file = open();
    let line = "Hello, World!\0";
    write_str(&mut file, line).unwrap();
    assert_eq!(read_str(&mut file).unwrap(), line);
}

#[test]
fn test_write_read_fifo_no_newline() {
    let mut file = open();
    for i in 0..10 {
        let line = format!("Write {i}");
        write_str(&mut file, &line).unwrap();
    }
    for i in 0..10 {
        let line = format!("Write {i}");
        assert_eq!(read_str(&mut file).unwrap(), line);
    }
    assert_eq!(
        read_str(&mut file).unwrap_err().kind(),
        io::ErrorKind::WouldBlock // Same as EAGAIN
    );
}

#[test]
fn test_read_empty() {
    let mut file = open();
    assert_eq!(
        read_str(&mut file).unwrap_err().kind(),
        io::ErrorKind::WouldBlock // Same as EAGAIN
    );
}

#[test]
fn test_write_too_long() {
    let mut file = open();
    let line = "A".repeat(MAX_STRING_LENGTH + 1);
    let result = write_str(&mut file, &line);
    assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidInput);
    // Should still be empty
    assert_eq!(
        read_str(&mut file).unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );

    let line = "A".repeat(MAX_STRING_LENGTH);
    write_str(&mut file, &line).unwrap();
    assert_eq!(read_str(&mut file).unwrap(), line);

    let line = "A".repeat(MAX_STRING_LENGTH - 1);
    write_str(&mut file, &line).unwrap();
    assert_eq!(read_str(&mut file).unwrap(), line);
}

#[test]
fn test_write_too_long_all_null() {
    let mut file = open();
    let line = "\0".repeat(MAX_STRING_LENGTH + 1);
    let result = write_str(&mut file, &line);
    assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidInput);
    // Should still be empty
    assert_eq!(
        read_str(&mut file).unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );

    let line = "\0".repeat(MAX_STRING_LENGTH);
    write_str(&mut file, &line).unwrap();
    assert_eq!(read_str(&mut file).unwrap(), line);

    let line = "\0".repeat(MAX_STRING_LENGTH - 1);
    write_str(&mut file, &line).unwrap();
    assert_eq!(read_str(&mut file).unwrap(), line);
}

#[test]
fn test_write_too_long_last_null() {
    let mut file = open();
    let line = "A".repeat(MAX_STRING_LENGTH) + "\0";
    let result = write_str(&mut file, &line);
    assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidInput);
    // Should still be empty
    assert_eq!(
        read_str(&mut file).unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );

    let line = "A".repeat(MAX_STRING_LENGTH - 1) + "\0";
    write_str(&mut file, &line).unwrap();
    assert_eq!(read_str(&mut file).unwrap(), line);

    let line = "A".repeat(MAX_STRING_LENGTH - 2) + "\0";
    write_str(&mut file, &line).unwrap();
    assert_eq!(read_str(&mut file).unwrap(), line);
}

#[test]
fn test_write_too_long_first_null() {
    let mut file = open();
    let line = "\0".to_owned() + &"A".repeat(MAX_STRING_LENGTH);
    let result = write_str(&mut file, &line);
    assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidInput);
    // Should still be empty
    assert_eq!(
        read_str(&mut file).unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );

    let line = "\0".to_owned() + &"A".repeat(MAX_STRING_LENGTH - 1);
    write_str(&mut file, &line).unwrap();
    assert_eq!(read_str(&mut file).unwrap(), line);

    let line = "\0".to_owned() + &"A".repeat(MAX_STRING_LENGTH - 2);
    write_str(&mut file, &line).unwrap();
    assert_eq!(read_str(&mut file).unwrap(), line);
}

#[test]
fn test_write_too_long_with_null() {
    let mut file = open();
    let line = {
        let filler = "A".repeat(MAX_STRING_LENGTH / 2 - 1);
        format!("{filler}\0{filler}A")
    };
    let result = write_str(&mut file, &format!("{line}A"));
    assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidInput);
    // Should still be empty
    assert_eq!(
        read_str(&mut file).unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );

    write_str(&mut file, &line).unwrap();
    assert_eq!(read_str(&mut file).unwrap(), line);
}

#[test]
fn test_write_too_many() {
    let mut file = open();
    let line = "Hello, World!";
    for _ in 0..MAX_MESSAGES {
        write_str(&mut file, line).unwrap();
    }
    let result = write_str(&mut file, line);
    assert_eq!(result.unwrap_err().raw_os_error(), Some(16)); // EBUSY, unstable API

    for _ in 0..MAX_MESSAGES {
        assert_eq!(read_str(&mut file).unwrap(), line);
    }
    assert_eq!(
        read_str(&mut file).unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );
}

#[test]
fn test_write_too_many_max_length() {
    let mut file = open();
    let line = "A".repeat(MAX_STRING_LENGTH);
    for _ in 0..MAX_MESSAGES {
        write_str(&mut file, &line).unwrap();
    }
    let result = write_str(&mut file, &line);
    assert_eq!(result.unwrap_err().raw_os_error(), Some(16)); // EBUSY, unstable API

    for _ in 0..MAX_MESSAGES {
        assert_eq!(read_str(&mut file).unwrap(), line);
    }
    assert_eq!(
        read_str(&mut file).unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );
}

#[test]
fn test_write_lots_fifo() {
    let mut file = open();

    for _ in 0..5 {
        for i in 0..(MAX_MESSAGES / 2 - 1) {
            write_str(&mut file, &i.to_string()).unwrap();
        }
        for i in 0..(MAX_MESSAGES / 2 - 1) {
            assert_eq!(read_str(&mut file).unwrap(), i.to_string());
        }
        assert_eq!(
            read_str(&mut file).unwrap_err().kind(),
            io::ErrorKind::WouldBlock
        );
    }
    assert_eq!(
        read_str(&mut file).unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );
}

#[test]
fn test_write_lots_max_length() {
    let mut file = open();
    let line = "A".repeat(MAX_STRING_LENGTH);

    for _ in 0..5 {
        for _ in 0..MAX_MESSAGES {
            write_str(&mut file, &line).unwrap();
        }
        let result = write_str(&mut file, &line);
        assert_eq!(result.unwrap_err().raw_os_error(), Some(16)); // EBUSY, unstable API

        for _ in 0..MAX_MESSAGES {
            assert_eq!(read_str(&mut file).unwrap(), line);
        }
        assert_eq!(
            read_str(&mut file).unwrap_err().kind(),
            io::ErrorKind::WouldBlock
        );
    }

    // Queue size = 0
    for _ in 0..(MAX_MESSAGES / 2) {
        write_str(&mut file, &line).unwrap();
    }
    // Queue size = MAX_MESSAGES / 2
    for _ in 0..(MAX_MESSAGES / 2 - 1) {
        assert_eq!(read_str(&mut file).unwrap(), line);
    }
    // Queue size = 1
    for _ in 0..(MAX_MESSAGES / 2) {
        write_str(&mut file, &line).unwrap();
    }
    // Queue size = MAX_MESSAGES / 2 + 1
    for _ in 0..(MAX_MESSAGES / 2 + 1) {
        assert_eq!(read_str(&mut file).unwrap(), line);
    }
    assert_eq!(
        read_str(&mut file).unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );
}

#[test]
fn test_empty_after_reading() {
    let mut file = open();
    write_str(&mut file, "Hello, World!").unwrap();
    assert_eq!(read_str(&mut file).unwrap(), "Hello, World!");
    assert_eq!(
        read_str(&mut file).unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );
}

#[test]
fn test_write_read_invalid_utf_8() {
    let mut file = open();
    let bytes = vec![0xC0];
    write_bytes(&mut file, &bytes).unwrap();
    assert_eq!(read_bytes(&mut file).unwrap(), bytes);
}

#[test]
fn test_write_read_bytes_null() {
    let mut file = open();
    let bytes = vec![0xC0, 0x00, 0xC1];
    write_bytes(&mut file, &bytes).unwrap();
    assert_eq!(read_bytes(&mut file).unwrap(), bytes);
}

#[test]
fn test_open_thread() {
    let mut file = open();

    let handles = (0..10)
        .map(|thread_number| {
            thread::spawn(move || {
                let mut file = open();
                thread::sleep(Duration::from_millis(thread_number * 100));
                write_str(
                    &mut file,
                    &format!("Hello, World from thread {thread_number}!"),
                )
                .unwrap();
                assert_eq!(
                    read_str(&mut file).unwrap(),
                    format!("Hello, World from thread {thread_number}!"),
                );
            })
        })
        .collect::<Vec<_>>();
    for handle in handles {
        handle.join().unwrap();
    }

    for thread_number in 0..10 {
        write_str(
            &mut file,
            &format!("Hello, World from thread {thread_number}!"),
        )
        .unwrap();
    }
    let handles = (0..10)
        .map(|thread_number| {
            thread::spawn(move || {
                let mut file = open();
                thread::sleep(Duration::from_millis(thread_number * 100));
                assert_eq!(
                    read_str(&mut file).unwrap(),
                    format!("Hello, World from thread {thread_number}!"),
                );
            })
        })
        .collect::<Vec<_>>();
    for handle in handles {
        handle.join().unwrap();
    }

    assert_eq!(
        read_str(&mut file).unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );
}

#[test]
fn test_threads_spam() {
    let mut file = open();

    let handles = (0..16)
        .map(|_| {
            thread::spawn(|| {
                let mut file = open();
                for _ in 0..10 {
                    for _ in 0..5 {
                        write_str(&mut file, "Hello, World!").unwrap();
                    }
                    for _ in 0..5 {
                        assert_eq!(read_str(&mut file).unwrap(), "Hello, World!");
                    }
                }
            })
        })
        .collect::<Vec<_>>();
    for handle in handles {
        handle.join().unwrap();
    }

    assert_eq!(
        read_str(&mut file).unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );
}

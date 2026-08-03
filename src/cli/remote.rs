pub(super) fn run_remote_command(args: &[String]) -> std::io::Result<i32> {
    match args.first().map(String::as_str) {
        Some("check") => remote_check(&args[1..]),
        Some("help" | "--help" | "-h") => {
            print_remote_help();
            Ok(0)
        }
        _ => {
            print_remote_help();
            Ok(2)
        }
    }
}

fn remote_check(args: &[String]) -> std::io::Result<i32> {
    let [target] = args else {
        eprintln!("usage: shepherd remote check <host>");
        return Ok(2);
    };

    match crate::remote::run_remote_check(target) {
        Ok(check) => {
            println!("{}", serde_json::to_string(&check)?);
            Ok(0)
        }
        Err(err) => {
            println!(
                "{}",
                serde_json::json!({
                    "ok": false,
                    "target": target,
                    "error": err.to_string(),
                })
            );
            crate::remote::print_remote_error_hint(&err, target);
            Ok(1)
        }
    }
}

fn print_remote_help() {
    eprintln!("shepherd remote commands:");
    eprintln!("  shepherd remote check <host>  verify peer identity and protocol");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_check_requires_exactly_one_host() {
        assert_eq!(remote_check(&[]).unwrap(), 2);
        assert_eq!(remote_check(&["a".into(), "b".into()]).unwrap(), 2);
    }
}

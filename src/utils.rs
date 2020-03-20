#[macro_use]
macro_rules! recv_until_command_or_end {
    ($rec:expr, $end:pat) => {
        async {
            if $rec.is_empty() {
                None
            } else {
                let mut return_command = None;

                while let Some(command) = $rec.recv().await {
                    match command {
                        $end => {
                            if $rec.is_empty() {
                                break;
                            } else {
                                continue;
                            }
                        },
                        _ => {
                            return_command = Some(command);
                            break
                        },
                    }
                }
                return_command
            } 
        }
    };
}

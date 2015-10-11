extern crate rusqlite;
extern crate getopts;
extern crate serial;

use getopts::Options;
use rusqlite::SqliteConnection;
use std::fmt;
use std::env;
use std::process::Command;
use std::io::Read;
use std::io;

struct Action {
	cmd: String,
	key: String,
}

impl fmt::Display for Action {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{}: {}", self.key, self.cmd)
	}
}

impl Action {
	fn new(cmd: &str,  key: &str) -> Action {
		Action {
			cmd: cmd.to_string(),
			key: key.to_string(),
		}
	}
	fn exec(self) {
		let cmd_status = Command::new("sh").arg("-c").arg(self.cmd).status().unwrap_or_else(|e| {
    			panic!("Failed to execute process: {}", e)
		});
		println!("process exited with: {}", cmd_status);
	}
}

fn print_usage(name: &str, opts: Options) {
	println!("Jukebox is a program which connects triggers e.g. RFID keys to actions e.g. playing an album.");
	let brief = format!("Usage:\t{} [options]", name);
	print!("{}", opts.usage(&brief));
}

fn main() {
    let args: Vec<String> = env::args().collect();
	let prog_name = args[0].clone();
    let mut prog_opts = Options::new();
    prog_opts.optflag("h", "help", "Print this usage information.");
    prog_opts.optflag("n", "new", "Start new database.");
    prog_opts.optflag("a", "add", "Add mode, add new action triggers to database.");
    prog_opts.optopt("f", "database", "Suggest a name for the database file default ./jukebox.db", "PATH");
    prog_opts.optopt("p", "port", "Serial port to use default /dev/ttyACM0", "PATH");
    let prog_opts_matches = match prog_opts.parse(&args[1..]) {
    	Ok(m) => { m }
    	Err(f) => { panic!(f.to_string()) }
    };
    if prog_opts_matches.opt_present("h") {
    	print_usage(&prog_name, prog_opts);
    	return;
    }
    let mut serr = match prog_opts_matches.opt_str("p") {
    	Some(p) => { serial::open(&p).unwrap() },
    	None	=> { serial::open("/dev/ttyACM0").unwrap() },
    };
    let db_file = match prog_opts_matches.opt_str("f") {
    	Some(f) => { f },
    	None	=> "./jukebox.db".to_string(),
    };
    let conn = SqliteConnection::open(db_file).unwrap();
    if prog_opts_matches.opt_present("n") {
	    conn.execute("CREATE TABLE jukebox (
			cmd	TEXT NOT NULL,
        	key	TEXT KEY
		)", &[]).unwrap();
		return;
	} else if prog_opts_matches.opt_present("a") {
		loop {
			let mut cmd = String::new();
			println!("Tap card on reader then enter command.\nCtrl+C to exit.");
			io::stdin().read_line(&mut cmd)
        		.ok()
        		.expect("Failed to read line");
			let mut input = String::new();
			serr.read_to_string(&mut input);
			if input.is_empty() {
				continue;
			}
			input = input[3..].to_string();
			input.truncate(10);
    		conn.execute("INSERT INTO jukebox (cmd, key) VALUES ($1, $2)", &[&cmd, &input]).unwrap();
    		println!("Action added command: {}, trigger: {}.", cmd, input);
    	}
    	return;
	}
	loop {
		let mut input = String::new();
		serr.read_to_string(&mut input);
		if input.is_empty() {
			continue;
		}
		input = input[3..].to_string();
		input.truncate(10);
		println!("Serial device said {}.", input);
	    let mut sql_req = conn.prepare("SELECT cmd, key FROM jukebox WHERE key = (?)").unwrap();
		let mut action_iter = sql_req.query_map(&[&input], |row| {
	    	Action {
				cmd: row.get(0),
				key: row.get(1)
			}
		}).unwrap();
		for action in action_iter {
			match action {
				Ok(trigger) => { 
					println!("Found match: {}.", trigger);
					trigger.exec();
				},
				Err(_) => { println!("Not an action.") },
			}
		};
	}
}

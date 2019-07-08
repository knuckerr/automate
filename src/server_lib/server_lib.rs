
extern crate regex;

extern crate serde;
extern crate serde_json;

use std::process::exit;
use crate::database::database::DatabaseQuerys;

use std::io;
use std::io::Read;
use std::fs::File;
use self::regex::Regex;


#[derive(Debug, Clone)]
pub struct Ports {
    pub protocol: String,
    pub port: String,
    pub stage: String,
    pub service: String,
    pub version: String,
}


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Target {
    pub ip: String,
    pub name: String,
    pub command: String,
    #[serde(skip)]
    pub id: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Commands {
    pub commands: Vec<CommandRun>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CommandRun {
    pub service: String,
    pub command: String,
    pub name: String,
    pub logfile: String,
    pub args: Vec<String>,
    #[serde(skip)]
    pub output: String,
    #[serde(skip)]
    pub id: i32,

    #[serde(skip)]
    pub state: String,
}


pub struct Enumaration;
impl Enumaration {
    pub fn start(output: &str, target: &Target) -> Vec<CommandRun> {
        let ports = Nmap::create(&output, target);
        let commands = Commands::new();
        match commands {
            Ok(mut n) => CommandRun::compare(&ports, &mut n, &target),
            Err(_n) => vec![],
        }
    }
}

//Create a struct of Ports by reading nmap output grabs the ports by reqex
//and after use again regex to catch the version of the services
//and returns it in to a vec of ports
pub struct Nmap;
impl Nmap {
    pub fn create(output: &str, target: &Target) -> Vec<Ports> {
        let mut ports = Vec::new();
        let re = Regex::new("([0-9]+/+[a-z].+(open|closed|filtered)+.*)");
        if re.is_err() {
            println!("{}", re.unwrap_err());
            exit(1);
        }
        let re = re.unwrap();
        for cap in re.captures_iter(&output) {
            let fields: Vec<_> = cap[0].split_whitespace().collect();
            let port_tcp: Vec<_> = fields[0].split('/').collect();

            let mut version_file = String::new();

            let r2 = Regex::new(r#"[A-Z].+\s+[0-9]+\.+([0-9]+.*)"#).unwrap();
            match r2.captures(&cap[0]) {
                Some(caps) => {
                    version_file.push_str(&caps[0]);
                }
                _ => {
                    version_file.push_str("Uknown");
                }
            }
            let tmp_port = Ports {
                protocol: port_tcp[1].to_string(),
                port: port_tcp[0].to_string(),
                stage: fields[1].to_string(),
                service: fields[2].to_string(),
                version: version_file,
            };

            ports.push(tmp_port);
        }
        DatabaseQuerys::insert_services(&ports, target);
        ports
    }
}



//Reads the commands json file and use the serde_json function to create a struct of commands
//if the commands.json file is not exist then is critical error
impl Commands {
    pub fn new() -> Result<Commands, serde_json::Error> {
        let json_file = read_file("src/comands.json");
        if json_file.is_err() {
            println!("{}", json_file.unwrap_err());
            exit(1);
        }
        let command: Commands = serde_json::from_str(&json_file.unwrap())?;
        Ok(command)

    }
}

//after we have our structs of nmap output and the struct from the json file
//we need to see what commands we need to run from the json file
//compare it to nmap results
impl CommandRun {
    pub fn compare(ports: &[Ports], commands: &mut Commands, target: &Target) -> Vec<CommandRun> {
        let mut commands_to_run = Vec::new();
        for command in &mut commands.commands {
            for port in ports.iter() {
                if command.service == port.service {
                    CommandRun::replace_args(port, &mut command.args, &target.to_owned());
                    commands_to_run.push(command.to_owned());
                }
            }
        }
        commands_to_run
    }

    //in our command json file we have some parts we need to replace
    //$target,$port etc
    //to make our command suite for our target
    pub fn replace_args(port: &Ports, args: &mut Vec<String>, target: &Target) {
        for arg in args {
            if arg == "$port" {
                *arg = port.port.to_owned();
            } else if arg == "$target" {
                *arg = target.ip.to_owned();
            }
        }
    }
}

pub fn read_file(path: &str) -> Result<String, io::Error> {
    let mut data = String::new();
    File::open(path)?.read_to_string(&mut data)?;
    Ok(data)
}


use std::{
    env::current_dir,
    fs,
    process::{self},
};

use chrono::{DateTime, Local, NaiveDateTime, TimeZone};
use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    #[command(help = "Subcommand to execute")]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Initalises planner")]
    Init(InitArgs),

    #[command(about = "List all current tasks")]
    List,

    #[command(about = "Adds a task")]
    Add(AddArgs),

    #[command(about = "Removes a task")]
    Rm(RmArgs),

    #[command(about = "Marks a task as complete")]
    Check(CheckArgs),
}

#[derive(Args)]
struct InitArgs {
    #[arg(help = "The directory where planner should be initialized")]
    dir: Option<String>,
}

#[derive(Args)]
struct AddArgs {
    #[arg(help = "The name of the task")]
    taskname: String,

    #[arg(help = "How many points the task should reward")]
    points_worth: u32,

    #[arg(help = "Due date of the task, given in the format 'yyyy-mm-dd HH:MM:SS'")]
    due_date: Option<String>,
}

#[derive(Args)]
struct RmArgs {
    #[arg(help = "The id of the task")]
    task_id: usize,
}

#[derive(Args)]
struct CheckArgs {
    #[arg(help = "The id of the task")]
    task_id: usize,
}

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct TaskList {
    tasks: Vec<Task>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Task {
    name: String,
    points: u32,
    id: usize,
    complete: bool,
    due_date: Option<DateTime<Local>>,
}

fn get_task_list() -> TaskList {
    let cwd = current_dir().unwrap();
    let mut meta_path = cwd.clone();
    meta_path.push("planner");
    meta_path.set_extension("json");

    if !meta_path.exists() {
        println!("Meta file does not exist, use 'planner init' to create it");
        process::exit(1);
    }

    let raw_file = fs::read_to_string(meta_path.clone()).unwrap();

    let task_list: TaskList = serde_json::from_str(raw_file.as_str()).unwrap();

    return task_list;
}

fn get_time_from_string(date: String) -> DateTime<Local> {
    let native = NaiveDateTime::parse_from_str(date.as_str(), "%Y-%m-%d %H:%M:%S").unwrap();
    let actual: DateTime<Local> = Local.from_local_datetime(&native).unwrap();

    return actual;
}

fn main() {
    let cli = Cli::parse();

    let cwd = current_dir().unwrap();
    let mut meta_path = cwd.clone();
    meta_path.push("planner");
    meta_path.set_extension("json");

    match cli.command {
        Commands::Init(args) => {
            let mut dir = cwd.into_os_string().into_string().unwrap();

            if let Some(x) = args.dir {
                dir = x;
            }

            let initial = TaskList { tasks: vec![] };

            fs::write(meta_path, serde_json::to_string(&initial).unwrap())
                .expect("Could not write to file");

            println!("Initialized planner in directory: {dir}");
        }
        Commands::Add(args) => {
            let mut task_list = get_task_list();

            let mut id = 0 as usize;

            loop {
                let mut found = false;

                for i in 0..task_list.tasks.len() {
                    if id == task_list.tasks[i].id {
                        found = true;
                        break;
                    }
                }

                if !found {
                    break;
                }

                id += 1;
            }

            let mut deadline: Option<DateTime<Local>> = None;

            if let Some(x) = args.due_date {
                deadline = Some(get_time_from_string(x));
            }

            let new_task = Task {
                name: args.taskname.clone(),
                points: args.points_worth,
                id: id,
                complete: false,
                due_date: deadline,
            };

            task_list.tasks.push(new_task);

            fs::write(meta_path, serde_json::to_string(&task_list).unwrap())
                .expect("Could not write to file");

            println!("Added task '{}'", args.taskname)
        }
        Commands::Rm(args) => {
            let mut task_list = get_task_list();

            let mut name: String = "".to_string();

            for i in 0..task_list.tasks.len() {
                if task_list.tasks[i].id == args.task_id {
                    name = task_list.tasks[i].name.clone();
                    task_list.tasks.remove(i);
                    break;
                }
            }

            if name == "" {
                println!("Task not found");
                return;
            }

            fs::write(meta_path, serde_json::to_string(&task_list).unwrap())
                .expect("Could not write to file");

            println!("Removed task '{name}'")
        }
        Commands::Check(args) => {
            let mut task_list = get_task_list();

            let mut name: String = "".to_string();

            for i in 0..task_list.tasks.len() {
                if task_list.tasks[i].id == args.task_id {
                    name = task_list.tasks[i].name.clone();
                    task_list.tasks[i].complete = true;
                    break;
                }
            }

            if name == "" {
                println!("Task not found");
                return;
            }

            fs::write(meta_path, serde_json::to_string(&task_list).unwrap())
                .expect("Could not write to file");

            println!("Checked off task '{name}'")
        }
        Commands::List => {
            let task_list = get_task_list();

            if task_list.tasks.len() > 0 {
                println!("Tasks:");

                let mut totpoints = 0;
                let mut allpoints = 0;

                for i in task_list.tasks {
                    let mut msg = format!(" #{} {} ({} points)", i.id, i.name, i.points);

                    if let Some(x) = i.due_date {
                        msg += format!(" due for {}", x.format("%Y-%m-%d at %H:%M:%S")).as_str();

                        let timeleft = x - Local::now();

                        let urgency = format!(
                            " ({}d {}h {}m left)",
                            timeleft.num_days(),
                            timeleft.num_hours() - (timeleft.num_days() * 24),
                            timeleft.num_minutes()
                                - (timeleft.num_hours() * 60)
                                - (timeleft.num_days() * 1440)
                        );
                        msg = msg + &urgency;
                    }

                    allpoints += i.points;

                    if i.complete {
                        totpoints += i.points;

                        println!("\x1b[32m{msg}\x1b[0m");
                    } else {
                        println!("{msg}");
                    }
                }

                let perc = ((totpoints as f32) / (allpoints as f32) * 100.0) as u32;

                if perc == 0 {
                    println!("\x1b[31mTotal points: {totpoints} ({}%)\x1b[0m", perc)
                } else if perc == 100 {
                    println!("\x1b[32mTotal points: {totpoints} ({}%)\x1b[0m", perc)
                } else {
                    println!("\x1b[33mTotal points: {totpoints} ({}%)\x1b[0m", perc)
                }
            } else {
                println!("No tasks added")
            }
        }
    }
}

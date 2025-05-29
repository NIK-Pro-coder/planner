use std::{env::current_dir, fs, process::exit};

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
    #[arg(long, short)]
    points: u32,

    #[arg(help = "Due date of the task, given in the format 'yyyy-mm-dd HH:MM:SS'")]
    #[arg(long)]
    due_date: Option<String>,

    #[arg(help = "Start time of the task, given in the format 'yyyy-mm-dd HH:MM:SS'")]
    #[arg(long)]
    start_time: Option<String>,

    #[arg(help = "A number representing how important the task is to complete")]
    #[arg(long, default_value_t = 0)]
    priority: u8,

    #[arg(help = "The id of the parent of this task")]
    #[arg(long)]
    parent_id: Option<usize>,

    #[arg(help = "A comma-separated list of resources to allocate to this task")]
    #[arg(long, short)]
    resources: Option<String>,
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

#[derive(Serialize, Deserialize, Debug, Clone)]
struct TaskList {
    tasks: Vec<Task>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Task {
    name: String,
    points: u32,
    id: usize,
    complete: bool,
    due_date: Option<DateTime<Local>>,
    start_time: Option<DateTime<Local>>,
    parent: Option<usize>,
    resources: Vec<String>,
}

fn get_task_list() -> TaskList {
    let cwd = current_dir().unwrap();
    let mut meta_path = cwd.clone();
    meta_path.push("planner");
    meta_path.set_extension("json");

    if !meta_path.exists() {
        println!("Meta file does not exist, use 'planner init' to create it");
        exit(1);
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

fn get_all_children_of_task(tasklist: &TaskList, parent: usize) -> Vec<Task> {
    let mut children: Vec<Task> = vec![];

    for i in 0..tasklist.tasks.len() {
        if let Some(x) = tasklist.tasks[i].parent {
            if x == parent {
                children.push(tasklist.tasks[i].clone());
            }
        }
    }

    return children;
}

fn task_has_children(tasklist: &TaskList, id: usize) -> bool {
    for i in 0..tasklist.tasks.len() {
        if tasklist.tasks[i].id == id {
            return get_all_children_of_task(&tasklist, i).len() > 0;
        }
    }

    return false;
}

fn get_start_and_end_of_children(
    children: Vec<Task>,
) -> (Option<DateTime<Local>>, Option<DateTime<Local>>) {
    let mut start_time: Option<DateTime<Local>> = None;
    let mut end_time: Option<DateTime<Local>> = None;

    for i in 0..children.len() {
        if let Some(x) = children[i].start_time {
            if start_time == None || x < start_time.unwrap() {
                start_time = Some(x);
            }
        }
        if let Some(x) = children[i].due_date {
            if end_time == None || x > end_time.unwrap() {
                end_time = Some(x);
            }
        }
    }

    return (start_time, end_time);
}

fn fit_task_size_to_children(
    tasklist: &mut TaskList,
    id: usize,
) -> (Option<DateTime<Local>>, Option<DateTime<Local>>) {
    println!("{id}");

    let mut children = get_all_children_of_task(tasklist, id);

    let mut start_time: Option<DateTime<Local>> = None;
    let mut end_time: Option<DateTime<Local>> = None;

    for i in 0..children.len() {
        let mut dates = (children[i].start_time, children[i].due_date);

        if task_has_children(tasklist, children[i].id) {
            dates = fit_task_size_to_children(tasklist, children[i].id);
            for j in 0..tasklist.tasks.len() {
                if tasklist.tasks[j].id == children[i].id {
                    tasklist.tasks[j].start_time = dates.0;
                    tasklist.tasks[j].due_date = dates.1;
                }
            }
        }

        if let Some(x) = dates.0 {
            if start_time == None || x < start_time.unwrap() {
                start_time = Some(x);
            }
        }
        if let Some(x) = dates.1 {
            if end_time == None || x > end_time.unwrap() {
                end_time = Some(x);
            }
        }
    }

    for i in 0..tasklist.tasks.len() {
        if tasklist.tasks[i].id == id {
            tasklist.tasks[i].start_time = start_time;
            tasklist.tasks[i].due_date = end_time;
        }
    }

    return (start_time, end_time);
}

fn print_task(i: &Task, indent: u8) {
    for _i in 0..indent {
        print!("  ");
    }

    let mut msg = format!("#{} {} ({} points)", i.id, i.name, i.points);

    if i.due_date == None && i.start_time != None {
        msg += format!(
            "  Start work on {}",
            i.start_time.unwrap().format("%Y-%m-%d at %H:%M:%S")
        )
        .as_str();
    } else if i.start_time == None && i.due_date != None {
        msg += format!(
            "  Due on {}",
            i.due_date.unwrap().format("%Y-%m-%d at %H:%M:%S")
        )
        .as_str();
    } else if i.start_time != None && i.due_date != None {
        msg += format!(
            "  Start work on {} and end on {}",
            i.start_time.unwrap().format("%Y-%m-%d at %H:%M:%S"),
            i.due_date.unwrap().format("%Y-%m-%d at %H:%M:%S")
        )
        .as_str();
    }

    if i.resources.len() > 0 {
        msg += "\n";
        for _i in 0..indent {
            msg += "  ";
        }

        msg += format!("Required resources: {:?}", i.resources).as_str();
    }

    if i.complete {
        println!("\x1b[32m{msg}\x1b[0m");
    } else {
        println!("{msg}");
    }
}

#[derive(Debug, Clone)]
struct TaskTreeNode {
    task: Option<usize>,
    children: Vec<usize>,
}

fn generate_task_tree(tasklist: &TaskList) -> Vec<TaskTreeNode> {
    let mut tree = vec![TaskTreeNode {
        task: None,
        children: vec![],
    }];

    let mut queue: Vec<usize> = (0..tasklist.tasks.len()).collect();

    loop {
        if queue.len() == 0 {
            break;
        }

        let t = queue.pop().unwrap();

        let mut proc = false;

        for i in 0..tree.len() {
            if tree[i].task == tasklist.tasks[t].parent {
                let l = tree.len();

                tree[i].children.push(l);
                tree.push(TaskTreeNode {
                    task: Some(tasklist.tasks[t].id),
                    children: vec![],
                });

                proc = true;
                break;
            }
        }

        if !proc {
            queue.insert(0, t);
        }
    }

    return tree;
}

fn print_task_tree(tasklist: &TaskList, tree: Vec<TaskTreeNode>, depth: u8, idx: usize) {
    let r = tree[idx].clone();

    let mut depth_add = 0;

    if let Some(x) = r.task {
        print_task(&tasklist.tasks[x], depth);
        depth_add = 1;
    }

    if r.children.len() > 0 {
        for i in r.children {
            print_task_tree(&tasklist, tree.clone(), depth + depth_add, i);
        }
    }
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

            let mut start_time: Option<DateTime<Local>> = None;

            if let Some(x) = args.start_time {
                start_time = Some(get_time_from_string(x));
            }

            if let Some(x) = args.parent_id {
                let mut found = false;
                for i in 0..task_list.tasks.len() {
                    if task_list.tasks[i].id == x {
                        found = true;
                        break;
                    }
                }

                if !found {
                    println!("Invalid parent id!");
                    exit(1);
                }
            }

            let mut new_vec: Vec<String> = vec![];

            if let Some(res) = args.resources {
                let old_vec: Vec<&str> = res.split(",").collect();

                for s in old_vec {
                    let new_str = s.trim().to_string();

                    if new_str != "" {
                        new_vec.push(new_str);
                    }
                }
            }

            let new_task = Task {
                name: args.taskname.clone(),
                points: args.points,
                id: id,
                complete: false,
                due_date: deadline,
                start_time: start_time,
                parent: args.parent_id,
                resources: new_vec,
            };

            task_list.tasks.push(new_task);

            println!("Added task '{}'", args.taskname);

            if let Some(x) = args.parent_id {
                println!("Fitting parent size to children");

                let mut actual_id = x;

                loop {
                    if task_list.tasks[actual_id].parent == None {
                        break;
                    }
                    actual_id = task_list.tasks[actual_id].parent.unwrap();
                }

                fit_task_size_to_children(&mut task_list, actual_id);
            }

            fs::write(meta_path, serde_json::to_string(&task_list).unwrap())
                .expect("Could not write to file");
        }
        Commands::Rm(args) => {
            let mut task_list = get_task_list();

            for i in 0..task_list.tasks.len() {
                if let Some(x) = task_list.tasks[i].parent {
                    if x == args.task_id {
                        task_list.tasks[i].parent = None;
                    }
                }
            }

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

            let children = get_all_children_of_task(&task_list, args.task_id);

            let mut cancomplete = true;

            for i in 0..children.len() {
                if !children[i].complete {
                    cancomplete = false;
                    break;
                }
            }

            if !cancomplete {
                println!("Cannot complete task, complete subtasks before!");
                exit(1);
            }

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

                for i in 0..task_list.tasks.len() {
                    allpoints += task_list.tasks[i].points;
                    if task_list.tasks[i].complete {
                        totpoints += task_list.tasks[i].points;
                    }
                }

                let tree = generate_task_tree(&task_list);

                print_task_tree(&task_list, tree, 1, 0);

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

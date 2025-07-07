use anyhow::{Error, anyhow};
use chrono::Utc;
use rusqlite::{Connection, Result, Row, Statement, params};
use std::result;

#[derive(Debug, PartialEq)]
enum Status {
    Pending,
    InProgress,
    Completed,
    Cancelled,
}

impl From<i64> for Status {
    fn from(i: i64) -> Self {
        match i {
            0 => Status::Pending,
            1 => Status::InProgress,
            2 => Status::Completed,
            3 => Status::Cancelled,
            _ => panic!(),
        }
    }
}

impl From<Status> for i64 {
    fn from(status: Status) -> Self {
        match status {
            Status::Pending => 0,
            Status::InProgress => 1,
            Status::Completed => 2,
            Status::Cancelled => 3,
        }
    }
}

pub struct Task {
    id: i64,
    task: String,
    status: Status,
    priority: i64,
    _created_at: i64,
    _due_at: Option<i64>,
}

impl std::fmt::Display for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let status_str = match self.status {
            Status::Pending => "Pending",
            Status::InProgress => "InProgress",
            Status::Completed => "Completed",
            Status::Cancelled => "Cancelled",
        };

        let priority_str = match self.priority {
            1 => ".",
            2 => "-",
            3 => "~",
            4 => "!",
            _ => "!!!",
        };

        write!(
            f,
            "{:<4} {:<11} [{:^3}]  \"{}\"",
            self.id, status_str, priority_str, self.task
        )
    }
}

pub fn print_task_header() {
    println!("{:<4} {:<11} {:<6} {}", "ID", "STATUS", "PRIO", "TASK")
}

impl TryFrom<&Row<'_>> for Task {
    type Error = rusqlite::Error;

    fn try_from(row: &Row) -> Result<Self> {
        Ok(Task {
            id: row.get(0)?,
            task: row.get(1)?,
            status: Status::from(row.get::<_, i64>(2)?),
            priority: row.get(3)?,
            _created_at: row.get(4)?,
            _due_at: row.get(5)?,
        })
    }
}

const TABLE_DDL: &str = "
    CREATE TABLE IF NOT EXISTS tasks (
        id INTEGER PRIMARY KEY,
        task TEXT NOT NULL,
        status INTEGER NOT NULL DEFAULT 0,
        priority INTEGER NOT NULL DEFAULT 3,
        created_at INT NOT NULL,
        due_at INT
    );";

pub fn init_db() -> Connection {
    let path = std::env::home_dir().unwrap().join(".cache/td");
    std::fs::create_dir_all(&path).unwrap();

    let conn = Connection::open(path.join("td.db")).expect("Unable to open database.");
    conn.execute(TABLE_DDL, [])
        .expect("Unable to create table in database.");

    conn
}

pub fn add_task(conn: &Connection, task: &str, priority: Option<i64>) {
    match conn.execute(
        "INSERT INTO tasks (task, priority, created_at) VALUES (?1, ?2, ?3);",
        params![task, priority.unwrap_or(3), Utc::now().timestamp()],
    ) {
        Ok(_) => println!("✓ Added task \"{}\"", task),
        Err(err) => println!("{:?}", err),
    }
}

fn select_to_tasks(statement: &mut Statement) -> Result<Vec<Task>> {
    statement
        .query_map([], |row| Task::try_from(row))?
        .collect()
}

pub fn list_tasks(conn: &Connection, all: bool, completed: bool) {
    let sql = match (all, completed) {
        (true, _) => "SELECT * FROM tasks;",
        (false, true) => "SELECT * FROM tasks WHERE status = 2;",
        (false, false) => {
            "SELECT * FROM tasks WHERE status IN (0, 1) ORDER BY status DESC, priority DESC;"
        }
    };

    let mut statement = conn.prepare(sql).expect("");

    match select_to_tasks(&mut statement) {
        Ok(tasks) => {
            print_task_header();
            tasks.iter().for_each(|task| println!("{task}"));
        }
        Err(_) => println!(""),
    }
}

fn update_task_status(conn: &Connection, id: i64, status: Status) -> result::Result<usize, Error> {
    match conn.execute(
        "UPDATE tasks SET status = ?1 WHERE id = ?2",
        [i64::from(status), id],
    ) {
        Ok(0) => Err(anyhow!("No rows were updated given id {id}")),
        Ok(n) => Ok(n),
        Err(e) => Err(e.into()),
    }
}

pub fn mark_task_done(conn: &Connection, id: i64) {
    match update_task_status(conn, id, Status::Completed) {
        Ok(_) => println!("Marked task [{id}] complete"),
        Err(err) => println!("{:?}", err),
    }
}

pub fn mark_task_pending(conn: &Connection, task: Task) {
    match update_task_status(conn, task.id, Status::Pending) {
        Ok(_) => println!("Paused task {}", task.id),
        Err(err) => println!("{:?}", err),
    }
}

pub fn mark_task_cancelled(conn: &Connection, id: i64) {
    match update_task_status(conn, id, Status::Cancelled) {
        Ok(_) => println!("Cancelled task {id}"),
        Err(err) => println!("{:?}", err),
    }
}

pub fn select_next_task(conn: &Connection, id: Option<i64>) {
    let next_id = match id {
        Some(id) => Ok(id),
        None => conn.query_row(
            "SELECT id
            FROM tasks
            WHERE status = 0
            ORDER BY priority DESC, due_at, created_at
            LIMIT 1;",
            [],
            |row| row.get(0),
        ),
    };

    match next_id {
        Ok(id) => match update_task_status(conn, id, Status::InProgress) {
            Ok(_) => println!("Set task {id} to in progress."),
            Err(_) => println!("No tasks to set to next."),
        },
        Err(rusqlite::Error::QueryReturnedNoRows) => println!("No tasks waiting. All done!"),
        Err(_) => panic!(),
    }
}

pub fn get_current_active_task(conn: &Connection) -> Option<Task> {
    match conn.query_row(
        "SELECT *
        FROM tasks
        WHERE status = 1
        LIMIT 1;",
        [],
        |row| Task::try_from(row),
    ) {
        Ok(task) => Some(task),
        Err(_) => None,
    }
}

#[cfg(test)]
fn init_test_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute(TABLE_DDL, []).unwrap();
    conn
}

#[cfg(test)]
fn get_single_task(conn: &Connection) -> Task {
    conn.query_row("SELECT * FROM tasks", [], |row| Task::try_from(row))
        .unwrap()
}

#[test]
fn test_add_tasks() {
    let conn = init_test_db();

    add_task(&conn, "Test task", None);

    let mut statement = conn.prepare("SELECT COUNT(*) FROM tasks").unwrap();
    let count: i64 = statement.query_row([], |row| row.get(0)).unwrap();
    assert_eq!(count, 1);

    add_task(&conn, "Test task", None);
    add_task(&conn, "Test task", None);
    add_task(&conn, "Test task", None);

    let mut statement = conn.prepare("SELECT COUNT(*) FROM tasks").unwrap();
    let count: i64 = statement.query_row([], |row| row.get(0)).unwrap();
    assert_eq!(count, 4);
}

#[test]
fn test_mark_done() {
    let conn = init_test_db();

    add_task(&conn, "Test task", None);
    mark_task_done(&conn, 1);

    let task = get_single_task(&conn);

    assert_eq!(task.status, Status::Completed)
}

#[test]
fn test_select_next_task() {
    let conn = init_test_db();

    add_task(&conn, "Test task", None);
    select_next_task(&conn, None);

    let task = get_single_task(&conn);

    assert_eq!(task.status, Status::InProgress)
}

#[test]
fn test_select_next_task_from_multiple() {
    let conn = init_test_db();

    add_task(&conn, "Test task", Some(1)); // id 1
    add_task(&conn, "Test task", None); // id 2
    add_task(&conn, "Test task", Some(5)); // id 3
    add_task(&conn, "Test task", Some(4)); // id 4

    select_next_task(&conn, None);

    let task = conn
        .query_row("SELECT * FROM tasks WHERE status = 1;", [], |row| {
            Task::try_from(row)
        })
        .unwrap();

    assert_eq!(task.status, Status::InProgress);
    assert_eq!(task.id, 3);
}

#[test]
fn test_no_next_task_to_select() {
    let conn = init_test_db();

    add_task(&conn, "Test task", None);
    mark_task_done(&conn, 1);
    select_next_task(&conn, None);

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM tasks WHERE status = 1;", [], |row| {
            row.get(0)
        })
        .unwrap();

    assert_eq!(count, 0)
}

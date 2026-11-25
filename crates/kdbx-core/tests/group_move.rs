//! Перенос группы целиком (Step 134): `move_group` переносит живую группу со всем содержимым под другую живую группу.
//!
//! Отказ — и ничего не меняется: перенос в свою подгруппу (цикл — поддерево отрывается от корня), в корзину и внутрь
//! неё, перенос корня, самой корзины и группы в корзине; в ту же группу и в себя. В корзину группа попадает только
//! удалением, не переносом.

mod common;

use kdbx_core::db_service::{self, NewDatabase};
use kdbx_core::error::Error;
use serde_json::json;
use uuid::Uuid;

const PASSWORD: &str = "group-move-1234";

struct Db(String);

impl Drop for Db {
    fn drop(&mut self) {
        let _ = db_service::close_kdbx(&self.0);
        let _ = std::fs::remove_file(&self.0);
    }
}

fn temp_path(name: &str) -> String {
    let mut p = std::env::temp_dir();
    p.push(format!("okp_group_move_{}_{}", name, std::process::id()));
    p.to_str().unwrap().to_string()
}

// KDF уменьшен, чтобы тест был быстрым в debug
fn create_db(name: &str) -> Db {
    common::init();
    let db_key = temp_path(&format!("{}.kdbx", name));
    let _ = std::fs::remove_file(&db_key);

    let mut v = serde_json::to_value(NewDatabase::default()).unwrap();
    v["database_name"] = json!("GroupMove");
    v["database_file_name"] = json!(db_key);
    v["password"] = json!(PASSWORD);
    v["kdf"] = json!({
        "algorithm": "Argon2id",
        "memory": 1024 * 1024,
        "iterations": 1,
        "parallelism": 1,
    });
    db_service::create_kdbx(serde_json::from_value(v).unwrap()).unwrap();
    Db(db_key)
}

fn root(db: &Db) -> Uuid {
    db_service::groups_summary_data(&db.0).unwrap().root_uuid
}

fn add_group(db: &Db, parent: Uuid, name: &str) -> Uuid {
    let group = db_service::new_blank_group_with_parent(parent, false).unwrap();
    let mut v = serde_json::to_value(&group).unwrap();
    v["name"] = json!(name);
    let uuid = Uuid::parse_str(v["uuid"].as_str().unwrap()).unwrap();
    db_service::insert_group(&db.0, serde_json::from_value(v).unwrap()).unwrap();
    uuid
}

fn add_entry(db: &Db, group: Uuid, title: &str) -> Uuid {
    let form = db_service::new_entry_form_data(Some(&group));
    let mut v = serde_json::to_value(&form).unwrap();
    common::form::set_field(&mut v, "Title", title);
    let uuid = Uuid::parse_str(v["uuid"].as_str().unwrap()).unwrap();
    db_service::insert_entry_from_form_data(&db.0, serde_json::from_value(v).unwrap()).unwrap();
    uuid
}

// Корзина с группой внутри: удаление записи создаёт корзину, группа в ней — как из чужого файла
fn bin_with_group(db: &Db) -> (Uuid, Uuid) {
    let binned = add_entry(db, root(db), "Binned");
    db_service::delete_entry(&db.0, binned).unwrap();
    let bin = db_service::groups_summary_data(&db.0)
        .unwrap()
        .recycle_bin_uuid;
    let inside = add_group(db, bin, "Inside");
    (bin, inside)
}

fn parent_of_group(db: &Db, group: Uuid) -> Uuid {
    let v = serde_json::to_value(db_service::get_group_by_id(&db.0, &group).unwrap()).unwrap();
    Uuid::parse_str(v["parent_group_uuid"].as_str().unwrap()).unwrap()
}

fn parent_of_entry(db: &Db, entry: Uuid) -> Uuid {
    let v = serde_json::to_value(db_service::get_entry_form_data_by_id(&db.0, &entry).unwrap())
        .unwrap();
    Uuid::parse_str(v["group_uuid"].as_str().unwrap()).unwrap()
}

// Группа видна из корня: путь вверх по родителям доходит до корня
fn reaches_root(db: &Db, group: Uuid) -> bool {
    let tree = db_service::groups_summary_data(&db.0).unwrap();
    let mut current = group;
    for _ in 0..=tree.groups.len() {
        if current == tree.root_uuid {
            return true;
        }
        match tree.groups.get(&current.to_string()) {
            Some(g) => current = g.parent_group_uuid,
            None => return false,
        }
    }
    false
}

fn save_and_reopen(db: &Db) {
    db_service::save_kdbx_with_backup(&db.0, None, false).unwrap();
    db_service::close_kdbx(&db.0).unwrap();
    db_service::load_kdbx(&db.0, Some(PASSWORD), None).unwrap();
}

fn assert_refused(result: kdbx_core::error::Result<()>, what: &str) {
    assert!(
        matches!(result, Err(Error::DataError(_))),
        "{}: {:?}",
        what,
        result
    );
}

#[test]
fn a_group_moves_with_all_it_holds() {
    let db = create_db("whole");
    let root = root(&db);
    let moved = add_group(&db, root, "Moved");
    let child = add_group(&db, moved, "Child");
    let entry = add_entry(&db, moved, "Entry");
    let deep = add_entry(&db, child, "Deep");
    let target = add_group(&db, root, "Target");

    db_service::move_group(&db.0, moved, target).unwrap();
    save_and_reopen(&db);

    assert_eq!(parent_of_group(&db, moved), target);
    assert_eq!(parent_of_group(&db, child), moved);
    assert_eq!(parent_of_entry(&db, entry), moved);
    assert_eq!(parent_of_entry(&db, deep), child);
    assert!(reaches_root(&db, child));
}

#[test]
fn a_group_moves_up_to_the_root() {
    let db = create_db("up");
    let root = root(&db);
    let parent = add_group(&db, root, "Parent");
    let moved = add_group(&db, parent, "Moved");

    db_service::move_group(&db.0, moved, root).unwrap();

    assert_eq!(parent_of_group(&db, moved), root);
}

#[test]
fn a_group_is_not_moved_into_its_own_subtree() {
    let db = create_db("cycle");
    let root = root(&db);
    let moved = add_group(&db, root, "Moved");
    let child = add_group(&db, moved, "Child");
    let grandchild = add_group(&db, child, "Grandchild");

    assert_refused(db_service::move_group(&db.0, moved, child), "into a child");
    assert_refused(
        db_service::move_group(&db.0, moved, grandchild),
        "into a grandchild",
    );
    assert_refused(db_service::move_group(&db.0, moved, moved), "into itself");

    assert_eq!(parent_of_group(&db, moved), root);
    assert_eq!(parent_of_group(&db, child), moved);
    assert!(reaches_root(&db, grandchild));
}

#[test]
fn a_group_is_not_moved_into_the_group_it_is_in() {
    let db = create_db("same");
    let root = root(&db);
    let moved = add_group(&db, root, "Moved");

    assert_refused(
        db_service::move_group(&db.0, moved, root),
        "into its parent",
    );
    assert_eq!(parent_of_group(&db, moved), root);
}

#[test]
fn a_group_is_not_moved_into_the_bin_or_a_group_in_it() {
    let db = create_db("into_bin");
    let root = root(&db);
    let moved = add_group(&db, root, "Moved");
    let (bin, inside) = bin_with_group(&db);

    assert_refused(db_service::move_group(&db.0, moved, bin), "into the bin");
    assert_refused(
        db_service::move_group(&db.0, moved, inside),
        "into a group in the bin",
    );
    assert_eq!(parent_of_group(&db, moved), root);
}

#[test]
fn the_root_the_bin_and_a_group_in_the_bin_are_not_moved() {
    let db = create_db("fixed");
    let root = root(&db);
    let target = add_group(&db, root, "Target");
    let (bin, inside) = bin_with_group(&db);

    assert_refused(db_service::move_group(&db.0, root, target), "the root");
    assert_refused(db_service::move_group(&db.0, bin, target), "the bin");
    assert_refused(
        db_service::move_group(&db.0, inside, target),
        "a group in the bin",
    );

    assert_eq!(parent_of_group(&db, bin), root);
    assert_eq!(parent_of_group(&db, inside), bin);
}

#[test]
fn an_unknown_uuid_is_not_found() {
    let db = create_db("unknown");
    let root = root(&db);
    let group = add_group(&db, root, "Group");

    assert!(matches!(
        db_service::move_group(&db.0, Uuid::new_v4(), root),
        Err(Error::NotFound(_))
    ));
    assert!(matches!(
        db_service::move_group(&db.0, group, Uuid::new_v4()),
        Err(Error::NotFound(_))
    ));
}

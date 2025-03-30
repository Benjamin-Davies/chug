// @generated automatically by Diesel CLI.

diesel::table! {
    dependencies (id) {
        id -> Integer,
        dependent_id -> Nullable<Integer>,
        dependency_id -> Integer,
    }
}

diesel::table! {
    downloaded_bottles (id) {
        id -> Integer,
        name -> Text,
        version -> Text,
        path -> Text,
    }
}

diesel::table! {
    linked_files (id) {
        id -> Integer,
        path -> Text,
        bottle_id -> Integer,
    }
}

diesel::joinable!(linked_files -> downloaded_bottles (bottle_id));

diesel::allow_tables_to_appear_in_same_query!(
    dependencies,
    downloaded_bottles,
    linked_files,
);

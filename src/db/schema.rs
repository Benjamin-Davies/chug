// @generated automatically by Diesel CLI.

diesel::table! {
    installed_bottles (id) {
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

diesel::joinable!(linked_files -> installed_bottles (bottle_id));

diesel::allow_tables_to_appear_in_same_query!(
    installed_bottles,
    linked_files,
);

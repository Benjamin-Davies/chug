// @generated automatically by Diesel CLI.

diesel::table! {
    installed_bottles (id) {
        id -> Integer,
        name -> Text,
        version -> Text,
        path -> Text,
    }
}

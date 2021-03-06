#![recursion_limit = "128"]
extern crate proc_macro;

use keg_functions::file_match_re;
use proc_macro::TokenStream;
use proc_macro2::{Span as Span2, TokenStream as TokenStream2};
use quote::quote;
use quote::ToTokens;
use std::ffi::OsStr;
use std::path::Path;
use std::{env, fs};
use syn::{parse_macro_input, Ident, LitStr};
use walkdir::{DirEntry, WalkDir};

lazy_static::lazy_static! {
    static ref RE: regex::Regex = file_match_re();
}

enum MigrationType {
    Mod,
    Sql,
}

fn find_migrations_file_names(
    location: Option<&Path>,
    mtype: MigrationType,
    full: bool,
) -> Vec<String> {
    //if parent dir was provided start from it, if not start from current dir
    let start = match location {
        Some(location) => fs::canonicalize(location).expect("invalid location provided"),
        None => env::current_dir().unwrap(),
    };

    WalkDir::new(start.as_path())
        .into_iter()
        .filter_map(Result::ok)
        .map(DirEntry::into_path)
        .filter(|entry| {
            entry
                .parent()
                .filter(|parent| {
                    //if parent_dir was not provided check if file is on a migrations dir
                    location.is_some() || parent.ends_with("migrations")
                })
                .is_some()
        })
        //filter entries which match the valid migration regex
        .filter(|entry| {
            entry
                .as_os_str()
                .to_str()
                .filter(|path| RE.is_match(path))
                .is_some()
        })
        //match the right extension
        .filter(|entry| {
            entry
                .extension()
                .and_then(OsStr::to_str)
                .filter(|ext| match mtype {
                    MigrationType::Mod => ext == &"rs",
                    MigrationType::Sql => ext == &"sql",
                })
                .is_some()
        })
        //if full is false get the name of the file without extension
        .filter_map(|entry| {
            if full {
                entry.into_os_string().into_string().ok()
            } else {
                entry
                    .file_stem()
                    .and_then(|file| file.to_os_string().into_string().ok())
            }
        })
        .collect()
}

fn migration_fn_quoted<T: ToTokens>(_migrations: Vec<T>) -> TokenStream2 {
    let result = quote! {
        use keg::{Migration, Runner};
        pub fn new() -> Runner {
            let quoted_migrations: Vec<(&str, String)> = vec![#(#_migrations),*];
            let mut migrations: Vec<Migration> = Vec::new();
            for module in quoted_migrations.into_iter() {
                migrations.push(Migration::from_filename(module.0, &module.1).unwrap());
            }
            Runner::new(&migrations)
        }
    };
    result
}

#[proc_macro]
pub fn include_migration_mods(input: TokenStream) -> TokenStream {
    let _location = if input.is_empty() {
        None
    } else {
        let location: LitStr = parse_macro_input!(input);
        Some(location.value())
    };

    let location = _location.as_ref().map(Path::new);
    let migration_mod_names = find_migrations_file_names(location, MigrationType::Mod, false);
    let mut migrations_mods = Vec::new();
    let mut _migrations = Vec::new();

    for migration in migration_mod_names.iter() {
        log::debug!("including mod {}", migration);

        let ident = Ident::new(migration, Span2::call_site());
        let mig_mod = quote! {pub mod #ident;};
        _migrations.push(quote! {(#migration, #ident::migration())});
        migrations_mods.push(mig_mod);
    }

    let fnq = migration_fn_quoted(_migrations);
    let result = quote! {
        #(#migrations_mods)*

        #fnq
    };
    result.into()
}

#[proc_macro]
pub fn embed_migrations(input: TokenStream) -> TokenStream {
    let _location = if input.is_empty() {
        None
    } else {
        let location: LitStr = parse_macro_input!(input);
        Some(location.value())
    };

    let location = _location.as_ref().map(Path::new);
    let migration_paths = find_migrations_file_names(location, MigrationType::Sql, true);
    let mut _migrations = Vec::new();
    for migration_path in migration_paths.iter() {
        let sql = fs::read_to_string(migration_path)
            .unwrap_or_else(|_| panic!("could not read migration {} content", migration_path));
        let migration_name = Path::new(migration_path)
            .file_stem()
            .and_then(|file| file.to_os_string().into_string().ok())
            .unwrap();
        _migrations.push(quote! {(#migration_name, #sql.into())});
    }

    let fnq = migration_fn_quoted(_migrations);
    (quote! {
        pub mod migrations {
            #fnq
        }
    })
    .into()
}

#[cfg(test)]
mod tests {
    use super::{find_migrations_file_names, migration_fn_quoted, MigrationType};
    use quote::quote;
    use std::fs;
    use std::path::Path;
    use tempdir::TempDir;

    #[test]
    fn finds_mod_migrations() {
        let tmp_dir = TempDir::new_in(".", "keg").unwrap();
        let _migrations_dir = fs::create_dir(tmp_dir.path().join("migrations")).unwrap();
        let mod1 = tmp_dir.path().join("migrations/V1__first.rs");
        fs::File::create(&mod1).unwrap();
        let mod2 = tmp_dir.path().join("migrations/V2__second.rs");
        fs::File::create(&mod2).unwrap();

        let mut mods = find_migrations_file_names(None, MigrationType::Mod, false);
        mods.sort();
        assert_eq!("V1__first", mods[0]);
        assert_eq!("V2__second", mods[1]);
    }

    #[test]
    fn finds_mod_migrations_in_parent_dir() {
        let tmp_dir = TempDir::new_in(".", "keg").unwrap();
        let mod1 = tmp_dir.path().join("V1__first.rs");
        fs::File::create(&mod1).unwrap();
        let mod2 = tmp_dir.path().join("V2__second.rs");
        fs::File::create(&mod2).unwrap();

        let mut mods = find_migrations_file_names(Some(tmp_dir.path()), MigrationType::Mod, false);
        mods.sort();
        assert_eq!("V1__first", mods[0]);
        assert_eq!("V2__second", mods[1]);
    }

    #[test]
    fn ignores_mod_files_without_migration_regex_match() {
        let tmp_dir = TempDir::new_in(".", "keg").unwrap();
        let _migrations_dir = fs::create_dir(tmp_dir.path().join("migrations")).unwrap();
        let mod1 = tmp_dir.path().join("migrations/V1first.rs");
        fs::File::create(&mod1).unwrap();
        let mod2 = tmp_dir.path().join("migrations/V2second.rs");
        fs::File::create(&mod2).unwrap();

        let mods = find_migrations_file_names(Some(tmp_dir.path()), MigrationType::Mod, false);
        assert!(mods.is_empty());
    }

    #[test]
    fn finds_sql_migrations() {
        let tmp_dir = TempDir::new_in("./", "keg").unwrap();
        let _migrations_dir = fs::create_dir(tmp_dir.path().join("migrations")).unwrap();
        let sql1 = tmp_dir.path().join("migrations/V1__first.sql");
        fs::File::create(&sql1).unwrap();
        let sql2 = tmp_dir.path().join("migrations/V2__second.sql");
        fs::File::create(&sql2).unwrap();

        let mut mods = find_migrations_file_names(None, MigrationType::Sql, true);
        mods.sort();
        assert_eq!(sql1, Path::new(&mods[0]));
        assert_eq!(sql2, Path::new(&mods[1]));
    }

    #[test]
    fn finds_sql_migrations_in_parent_dir() {
        let tmp_dir = TempDir::new_in("./", "keg").unwrap();
        let sql1 = tmp_dir.path().join("V1__first.sql");
        fs::File::create(&sql1).unwrap();
        let sql2 = tmp_dir.path().join("V2__second.sql");
        fs::File::create(&sql2).unwrap();

        let mut mods = find_migrations_file_names(Some(tmp_dir.path()), MigrationType::Sql, true);
        mods.sort();
        assert_eq!(sql1, Path::new(&mods[0]));
        assert_eq!(sql2, Path::new(&mods[1]));
    }

    #[test]
    fn ignores_sql_files_without_migration_regex_match() {
        let tmp_dir = TempDir::new("keg").unwrap();
        let _migrations_dir = fs::create_dir(tmp_dir.path().join("migrations")).unwrap();
        let sql1 = tmp_dir.path().join("migrations/V1first.sql");
        fs::File::create(&sql1).unwrap();
        let sql2 = tmp_dir.path().join("migrations/V2second.sql");
        fs::File::create(&sql2).unwrap();

        let mods = find_migrations_file_names(Some(tmp_dir.path()), MigrationType::Sql, true);
        assert!(mods.is_empty());
    }

    #[test]
    fn test_quote_fn() {
        let migs = vec![quote!("V1__first", "valid_sql_file")];
        let expected = concat! {
            "use keg :: { Migration , Runner } ; ",
            "pub fn new ( ) -> Runner { ",
            "let quoted_migrations : Vec < ( & str , String ) > = vec ! [ \"V1__first\" , \"valid_sql_file\" ] ; ",
            "let mut migrations : Vec < Migration > = Vec :: new ( ) ; ",
            "for module in quoted_migrations . into_iter ( ) { ",
            "migrations . push ( Migration :: from_filename ( module . 0 , & module . 1 ) . unwrap ( ) ) ; ",
            "} ",
            "Runner :: new ( & migrations ) }"
        };
        assert_eq!(expected, migration_fn_quoted(migs).to_string());
    }
}

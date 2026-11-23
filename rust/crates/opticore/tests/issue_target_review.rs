use opticore::resolver;
use std::fs;

#[test]
fn reported_palworld_and_hogwarts_paths_do_not_use_engine_folder() {
    for (folder, relative, bootstrap) in [
        (
            "Palworld",
            "Pal/Binaries/Win64/Palworld-Win64-Shipping.exe",
            "Palworld.exe",
        ),
        (
            "Hogwarts Legacy",
            "Phoenix/Binaries/Win64/HogwartsLegacy.exe",
            "HogwartsLegacy.exe",
        ),
    ] {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join(folder);
        let exe = root.join(relative);
        fs::create_dir_all(exe.parent().unwrap()).unwrap();
        fs::write(&exe, b"fixture").unwrap();
        let engine = root.join("Engine/Binaries/Win64");
        fs::create_dir_all(&engine).unwrap();
        fs::write(engine.join("CrashReportClient.exe"), b"fixture").unwrap();
        fs::write(root.join(bootstrap), b"fixture").unwrap();
        let target = match resolver::resolve(&root) {
            Ok(target) => target,
            Err(error) => {
                assert!(error.contains("choose the actual game EXE"), "{error}");
                resolver::remember(&root, &exe).unwrap();
                resolver::resolve(&root).unwrap()
            }
        };
        assert_eq!(target.executable, exe.canonicalize().unwrap(), "{folder}");
        assert_eq!(
            target.directory,
            exe.parent().unwrap().canonicalize().unwrap(),
            "{folder}"
        );
    }
}

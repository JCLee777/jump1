use std::{env, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=assets/app-icon.ico");

    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let version = env::var("CARGO_PKG_VERSION").expect("Cargo sets the package version");
    let parts = version
        .split('.')
        .map(|part| {
            part.parse::<u16>()
                .expect("Windows version parts are numeric")
        })
        .collect::<Vec<_>>();
    assert_eq!(parts.len(), 3, "Jump1 version must be MAJOR.MINOR.PATCH");

    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let icon_path = manifest_dir.join("assets/app-icon.ico");
    let icon_path = icon_path.to_string_lossy().replace('\\', "/");
    let version_info = format!("{}.{}.{}", parts[0], parts[1], parts[2]);
    let resource = format!(
        r#"
1 ICON "{icon_path}"

1 VERSIONINFO
 FILEVERSION {major},{minor},{patch},0
 PRODUCTVERSION {major},{minor},{patch},0
 FILEFLAGSMASK 0x3fL
 FILEFLAGS 0x0L
 FILEOS 0x40004L
 FILETYPE 0x1L
 FILESUBTYPE 0x0L
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "040904b0"
    BEGIN
      VALUE "CompanyName", "JCLee777\0"
      VALUE "FileDescription", "Jump1\0"
      VALUE "FileVersion", "{version_info}\0"
      VALUE "InternalName", "jump1\0"
      VALUE "OriginalFilename", "Jump1.exe\0"
      VALUE "ProductName", "Jump1\0"
      VALUE "ProductVersion", "{version_info}\0"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x409, 1200
  END
END
"#,
        major = parts[0],
        minor = parts[1],
        patch = parts[2],
    );

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let resource_path = out_dir.join("jump1.rc");
    fs::write(&resource_path, resource).expect("write Windows resources");
    embed_resource::compile(resource_path, embed_resource::NONE)
        .manifest_optional()
        .expect("embed Jump1 Windows icon and version resources");
}

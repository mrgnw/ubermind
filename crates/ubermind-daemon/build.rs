use std::path::Path;

fn main() {
	// Ensure the UI build directory exists so rust-embed doesn't fail.
	// When building from the workspace with a real UI build, this is a no-op.
	// When building from a crate tarball (cargo publish --dry-run), this
	// creates an empty directory so the derive macro succeeds.
	let ui_dir = Path::new("../../ui/build");
	if !ui_dir.exists() {
		let _ = std::fs::create_dir_all(ui_dir);
	}
}

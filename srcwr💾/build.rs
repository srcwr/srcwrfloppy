use extshared_build_helper::*;

fn main() {
	let mut build = smext_build();
	use_cellarray(&mut build);
	compile_lib(build, "smext");
}


build:
	cd core;cargo build --release;
	cp core/target/release/gencore plugin/gencore


build:
	cd core;cargo build --release;
	cp core/target/release/gencore src/gencore
	cd src;zip -r ../genanki-rs.ankiaddon __init__.py gencore manifest.json


build:
	cd core;maturin build --release --out pybuild;
	unzip -o core/pybuild/*.whl -d src/;
	mv src/gencore/*.so src/gencore.so;
	rm -rvf src/gencore src/genanki-*;
	cd src;zip -r ../genanki-rs.ankiaddon __init__.py gencore.so manifest.json user_files/config.toml

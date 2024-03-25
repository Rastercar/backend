
.PHONY: lazy
lazy:
	git add . && git commit -m "." && git push origin master
	
.PHONY: run_prod
run_prod:
	RUST_LOG=error cargo run

.PHONY: run_dev
run_dev:
	RUST_LOG=warn cargo watch -x run

.PHONY: run_dev_no_watch
run_dev_no_watch:
	RUST_LOG=warn cargo run

.PHONY: run_info
run_info:
	RUST_LOG=info cargo watch -x run

.PHONY: run_debug
run_debug:
	RUST_LOG=debug cargo watch -x run


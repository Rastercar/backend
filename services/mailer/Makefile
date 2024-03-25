# [PROD-TODO] remove me
.PHONY: lazy
lazy:
	git add . && git commit -m "." && git push origin master

.PHONY: run_dev
run_dev:
	RUST_LOG=info,aws_config=warn AWS_PROFILE=rastercar-vitor cargo watch -x run

.PHONY: run_debug
run_debug:
	RUST_LOG=debug AWS_PROFILE=rastercar-vitor cargo watch -x run
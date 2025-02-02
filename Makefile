# ---------------------- [GIT] ----------------------
.PHONY: lazy
lazy:
	git add . && git commit -m "." && git push origin master

# ---------------------- [MAILER] ----------------------
.PHONY: run_mailer_error
run_mailer_error:
	RUST_LOG=error cargo watch -x 'run -p mailer'

.PHONY: run_mailer_warn
run_mailer_warn:
	RUST_LOG=warn cargo watch -x 'run -p mailer'

.PHONY: run_mailer_info
run_mailer_info:
	RUST_LOG=info cargo watch -x 'run -p mailer' 

.PHONY: run_mailer_dev
run_mailer_dev:
	RUST_LOG=info cargo watch -x 'run -p mailer'

.PHONY: run_mailer_debug
run_mailer_debug:
	APP_DEBUG=true RUST_LOG=debug cargo watch -x 'run -p mailer'

# ---------------------- [DECODER] ----------------------
.PHONY: run_decoder_error
run_decoder_error:
	RUST_LOG=error cargo watch -x 'run -p decoder'

.PHONY: run_decoder_warn
run_decoder_warn:
	RUST_LOG=warn cargo watch -x 'run -p decoder'

.PHONY: run_decoder_info
run_decoder_info:
	RUST_LOG=info cargo watch -x 'run -p decoder' 

.PHONY: run_decoder_debug
run_decoder_debug:
	RUST_LOG=debug cargo watch -x 'run -p decoder'

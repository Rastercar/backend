# ---------------------- [GIT] ----------------------
.PHONY: lazy
lazy:
	git add . && git commit -m "." && git push origin homolog

# ---------------------- [MAILER] ----------------------
.PHONY: start_mailer_error
start_mailer_error:
	RUST_LOG=error cargo watch -x 'run -p mailer'

.PHONY: start_mailer_warn
start_mailer_warn:
	RUST_LOG=warn cargo watch -x 'run -p mailer'

.PHONY: start_mailer_info
start_mailer_info:
	RUST_LOG=info cargo watch -x 'run -p mailer' 

.PHONY: start_mailer_dev
start_mailer_dev:
	RUST_LOG=info cargo watch -x 'run -p mailer'

.PHONY: start_mailer_debug
start_mailer_debug:
	APP_DEBUG=true RUST_LOG=debug cargo watch -x 'run -p mailer'

# ---------------------- [DECODER] ----------------------
.PHONY: start_decoder_error
start_decoder_error:
	RUST_LOG=error cargo watch -x 'run -p decoder'

.PHONY: start_decoder_warn
start_decoder_warn:
	RUST_LOG=warn cargo watch -x 'run -p decoder'

.PHONY: start_decoder_info
start_decoder_info:
	RUST_LOG=info cargo watch -x 'run -p decoder' 

.PHONY: start_decoder_debug
start_decoder_debug:
	RUST_LOG=debug cargo watch -x 'run -p decoder'

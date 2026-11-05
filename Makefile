# smk { overwrite: false }

# smk's checks: format, lint, types and the commit hook, rewritten on every render. ci runs them
# through `checks`, which smkproj.toml's Checks chooses. The Rust builds and tests in the builder VM,
# not here: README "The Dojo VM".
include .smk/checks.mk

VERSION := $(shell cat VERSION)

help:
	@perl -nle'print $& if m{^[a-zA-Z_-]+:.*?## .*$$}' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'
.PHONY: help

tag-release:
	git tag $(VERSION)
	git push origin $(VERSION)

delete-release-tag:
	git tag -d $(VERSION)
	git push origin :$(VERSION)

re-tag-release: delete-release-tag tag-release

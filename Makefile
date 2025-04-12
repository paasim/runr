.PHONY: check clean clean-bare doc run test test-all test-hook
BARE_PATH=$(PWD)/tmp/runr-bare

check: .git/hooks/pre-commit
	. $<

clean:
	rm -rf target

clean-bare:
	rm -rf $(BARE_PATH)
	git remote rm bare || true # ensure the remote does not exist

doc:
	cargo doc --open

run: sync-bare
	BRANCH=main BARE_PATH=$(BARE_PATH) cargo run

test:
	cargo test

test-all:
	make clean-bare
	make $(BARE_PATH)
	git push bare main -f
	BRANCH=main BARE_PATH=$(BARE_PATH) cargo test -- \
		--include-ignored \
		--test-threads=1

test-hook:
	make clean-bare
	make $(BARE_PATH)/hooks/post-receive
	git push bare main -f

$(BARE_PATH):
	mkdir -p $@
	cd $@ && git init --bare
	git remote add bare $@

$(BARE_PATH)/hooks/post-receive: post-receive-hook $(BARE_PATH)
	cp $< $@

.git/hooks/pre-commit:
	curl -o $@ https://gist.githubusercontent.com/paasim/317a1fd91a6236ca36d1c1c00c2a02d5/raw/767f2ab0b59e6bf5fe5c44608a872c5293f6e64e/rust-pre-commit.sh
	echo "" >> $@
	chmod +x $@

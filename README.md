A useful wrapper for oracle scripts/interestingness tests in reducers such as creduce, or [preduce](https://github.com/fitzgen/preduce).

---

Experimental software - needs a large cleanup and refactor.

### TODO:

- [ ] Clean up arguments, remove no-longer-used arguments
- [ ] Implement features:
- * [ ] Stdout comparison (can already be done by comparing stdout hashes with builtins)
- * [ ] Exit code filtering - Allow to specify non-zero exit codes as interesting (workaround: Use the `$?` shell variable in `COMMAND`)


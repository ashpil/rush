# rush - the really uninteractive shell
Rush is designed to run POSIX scripts, and run them fast. Think of it as a Rust [dash](https://en.wikipedia.org/wiki/Almquist_shell#dash).

I know that the name is overused, but it's too good to pass up.

### // TODO: 
- [X] Simple command execution `ls -l`
- [X] Redirection `exa -1 | grep cargo`
- [X] Exit status logic ` ! false && ls || date`
- [ ] Redirection `date > time.txt`
- [ ] Async execution `&`
- [X] Shell builtins `cd` `exit`
- [ ] Environmental variables
- [ ] Variables
- [ ] Quotes
- [ ] Control flow `if` `for` `while` `case` etc
- [ ] Expand this to-do list

# focus_timer

This is my first hands on Rust to learn the lang.

## Commands

Show active task info

```bash
./focus_timer 
```

Show db path

```bash
./focus_timer info
```

Remove database

```bash
./focus_timer flush
```

Create new task

```bash
./focus_timer new -t "task name"
```

Operations on tasks (start, stop, complete, delete)

```bash
./focus_timer start -i 1
./focus_timer stop -i 1
./focus_timer complete -i 1
./focus_timer delete -i 1
```

List N tasks over the period

```bash
./focus_timer list
./focus_timer list --date_from 2025-01-01
./focus_timer list --date_from 2025-01-01 --date_to 2025-01-01 -n 10
```

Show stat over the period

```bash
./focus_timer stat
./focus_timer stat --date_from 2025-01-01
```

Export to csv

```bash
./focus_timer export --path <path_to_csv>
./focus_timer export --date_from 2025-01-01 --date_to 2025-01-01 --path <path_to_csv>
```

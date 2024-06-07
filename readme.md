# rocksdb-flush-safety

This repository contains a simple test program that demonstrates an issue with RocksDB flush safety in absence of write ahead log (WAL).

If the WAL is disabled, RocksDB may lose data if the process crashes even if the data has been flushed. 

## Test case

This program writes two keys in different column families and flush them in a specific order.

- write key in CF1
- write key in CF2
- maybe crash
- flush CF1
- flush CF2
- maybe crash

The write operations are not `sync` therefore this ordering ensure that the key must exist in CF1 IF it exists in CF2.

To witness the issue, run in two terminals:

```bash
./test-loop-no-wal
```

and

```bash
./test-loop-wal
```

You can expect the `test-loop-wal` to run forever without any issue.

However, the `test-loop-no-wal` will eventually fail with the following message (with a different key):

```
thread 'main' panicked at src/main.rs:47:13:
Inconsistent flushing - mapping exists but no vector found for key: test_key-208393
stack backtrace:
   0:     0x58e815ab1a42 - <std::sys_common::backtrace::_print::DisplayBacktrace as core::fmt::Display>::fmt::hffecb437d922f988
   1:     0x58e815ad538c - core::fmt::write::hd9a8d7d029f9ea1a
   2:     0x58e815aaf45f - std::io::Write::write_fmt::h0e1226b2b8d973fe
   3:     0x58e815ab1814 - std::sys_common::backtrace::print::he907f6ad7eee41cb
   4:     0x58e815ab2ccb - std::panicking::default_hook::{{closure}}::h3926193b61c9ca9b
   5:     0x58e815ab2a23 - std::panicking::default_hook::h25ba2457dea68e65
   6:     0x58e815ab31dd - std::panicking::rust_panic_with_hook::h0ad14d90dcf5224f
   7:     0x58e815ab30b2 - std::panicking::begin_panic_handler::{{closure}}::h4a1838a06f542647
   8:     0x58e815ab1f16 - std::sys_common::backtrace::__rust_end_short_backtrace::h77cc4dc3567ca904
   9:     0x58e815ab2de4 - rust_begin_unwind
  10:     0x58e815ad4495 - core::panicking::panic_fmt::h940d4fd01a4b4fd1
  11:     0x58e8155292ac - rocksdb_flush_safety::main::hfa9f0284dc75177a
  12:     0x58e815535083 - std::sys_common::backtrace::__rust_begin_short_backtrace::hcb8eeac690801cd8
  13:     0x58e8155329d9 - std::rt::lang_start::{{closure}}::h1cbb91236288c970
  14:     0x58e815aab2b3 - std::rt::lang_start_internal::h103c42a9c4e95084
  15:     0x58e8155298b5 - main
  16:     0x7d0b0202a1ca - __libc_start_call_main
                               at ./csu/../sysdeps/nptl/libc_start_call_main.h:58:16
  17:     0x7d0b0202a28b - __libc_start_main_impl
                               at ./csu/../csu/libc-start.c:360:3
  18:     0x58e815520085 - _start
  19:                0x0 - <unknown>
```

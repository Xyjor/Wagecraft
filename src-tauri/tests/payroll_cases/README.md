# Golden payroll cases

Each JSON file here is one employee's semi-monthly payslip with the expected amounts
worked out by hand (plan §7.8, §11.2). `cargo test golden` runs them all against the
PH-2026 rule pack and lists every difference.

## Adding a case

1. Copy the closest existing case and give it the next number.
2. Describe the employee and the period:
   - `employee`: `basis` (`monthly` or `daily`), `rate` in pesos as a string,
     `daysPerWeek` (5 = Mon–Fri, 6 = Mon–Sat), `minimumWageEarner`.
   - `period`: `from`, `to` and `cutoff` (1 or 2).
3. Say only what differs from a normal cutoff. By default every work day is an ordinary
   day worked in full (8 hours) and every other day is a rest day off.
   - `days`: changes by date. Each can set `dayType` (`ordinary`, `restDay`, `special`,
     `specialRestDay`, `regular`, `regularRestDay`, `doubleRegular`,
     `doubleRegularRestDay`), `status` (`worked`, `absent`, `paidLeave`, `unpaidLeave`,
     `off`), and minutes for `late`, `undertime`, `worked` and `night`.
   - `everyWorkDay`: one change for all work days, such as a night shift.
   - `overtime`: approved blocks with `dayType`, `minutes` and `night` minutes.
   - `allowances`: `label`, `amount` and `taxable`.
   - `deductions`: loans and other deductions in the order they are taken, each with
     `kind` (`loan` or `other`), `label` and `amount`. `minimumNet` is the lowest net
     pay they may leave (₱0 if left out).
4. Work out every line with a calculator from the plan's §7 rules and Appendix A tables.
   Write the steps in `workings` so someone else can redo them.
5. Put the results in `expected`. Lines are listed in payslip order. Deductions are negative,
   and lines that come to zero are left out. With deductions, also give `deductions` (the
   total taken) and `unpaid` (each label with the amount carried over).

Never change an expected value to make a failing case pass. If the expected value was
wrong, fix it in its own commit that explains why.

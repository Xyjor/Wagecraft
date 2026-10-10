# Manual test script

Run this on the built installer before each release tag (plan §11.3). It covers the seven flows
that must work end to end. Each flow lists the steps and what you should see after each one.
Fill in the results table at the end and keep it with the release notes.

Automated end-to-end tests are cut from v1, so this script is the only check that the installed
app, the real WebView and the real file system behave together.

## Before you start

- A Windows 10 or 11 PC that has never run Wagecraft, or a test PC where you removed
  `%APPDATA%\io.github.xyjor.wagecraft` (that folder holds the database, so never do this on the
  office PC).
- The installer from the GitHub Release you are testing.
- About 45 minutes.
- For flow 7 you also need a development build (`pnpm tauri dev` in the repository), because the
  installed app has no developer console.

Use the names and values below exactly, so the expected results match.

## Flow 1: first run, Admin, sign in and out

| Step | Do this                                                                 | Expect                                                                                   |
| ---- | ----------------------------------------------------------------------- | ---------------------------------------------------------------------------------------- |
| 1.1  | Run the installer.                                                      | SmartScreen may warn (More info, Run anyway). Install finishes without asking for admin. |
| 1.2  | Start Wagecraft.                                                        | The **Set up Wagecraft** screen opens.                                                   |
| 1.3  | Type company `Test Co`, username `admin`, password `admin` and confirm. | The form refuses: "Use at least 10 characters".                                          |
| 1.4  | Change the password to `correct horse` twice, press **Create Admin**.   | You are signed in. The sidebar shows Home through Settings.                              |
| 1.5  | Press **Sign out**.                                                     | The **Sign in** screen opens.                                                            |
| 1.6  | Sign in as `admin` with the wrong password five times.                  | Each try says the username or password is wrong. Nothing says the account is locked.     |
| 1.7  | Sign in with the right password straight away.                          | Still refused with the same message (the account is locked for 15 minutes).              |
| 1.8  | Close Wagecraft and start it again.                                     | The Sign in screen opens, not the setup wizard.                                          |

Step 1.7 locks the account, so wait 15 minutes before flow 2, or on a test PC remove the app data
folder and repeat 1.2 to 1.5 with the right password.

## Flow 2: HR sets up a department, a position and an employee

Sign in as `admin` (Admin has every HR permission).

| Step | Do this                                                                                                                                                                         | Expect                                                                                      |
| ---- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| 2.1  | Organization, **Add department**: code `OPS`, name `Operations`.                                                                                                                | Operations appears in the Departments table as Active.                                      |
| 2.2  | **Add position**: title `Driver`, department Operations, salary range 15,000 to 20,000.                                                                                         | Driver appears in the Positions table with its range.                                       |
| 2.2b | **Add schedule**: name `Day shift`, keep 8:00 to 17:00, Monday to Friday.                                                                                                       | Day shift appears in the Work schedules table.                                              |
| 2.3  | Employees, **Add employee**: number `EMP-0001`, first name `Juan`, last name `Dela Cruz`, status Regular, hire date two years ago, Operations, Driver, work schedule Day shift. | After saving, Juan's profile opens. Leave and lateness need the schedule, so don't skip it. |
| 2.4  | Compensation tab: basis Monthly, rate `18000`, starts on the 1st of this month, reason `Starting rate`.                                                                         | The rate appears with the Current badge.                                                    |
| 2.5  | Compensation tab again: rate `25000`, starts on the 1st of next month, reason `Test`.                                                                                           | Saved, with the note "₱25,000.00 is above the Driver range, which tops out at ₱20,000.00."  |
| 2.6  | Kiosk PIN tab: PIN `1234` twice, **Set PIN**.                                                                                                                                   | "PIN saved" shows, and the button now reads **Replace PIN**.                                |
| 2.7  | Sign-in account tab, **Create a Staff account**: username `juan`, temporary password `temporary1`.                                                                              | The tab shows the account, role Staff, status Active.                                       |
| 2.8  | Users.                                                                                                                                                                          | `juan` is listed as Staff with "Must change password", employee EMP-0001 Juan Dela Cruz.    |

## Flow 3: the time clock, then the record

| Step | Do this                                                    | Expect                                                                                            |
| ---- | ---------------------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| 3.1  | Sign out. On the Sign in screen press **Clock in or out**. | The **Time clock** screen shows today's date and a running clock.                                 |
| 3.2  | Employee no. `EMP-0001`, PIN `0000`, **Time in**.          | Refused: "Wrong employee number or PIN".                                                          |
| 3.3  | PIN `1234`, **Time in**.                                   | "Time in recorded, Juan." with the time.                                                          |
| 3.4  | **Time in** again with the same PIN.                       | Refused: "You already clocked in for this shift."                                                 |
| 3.5  | Wait a minute, then **Time out** with the same PIN.        | "Time out recorded, Juan."                                                                        |
| 3.6  | **Back to sign in**, sign in as `juan` with `temporary1`.  | The **Change password** screen opens first.                                                       |
| 3.7  | Set the password `juan passphrase 1`.                      | Juan is signed in. The sidebar shows only Home, My profile, My attendance, My leave, My payslips. |
| 3.8  | Open **My attendance**.                                    | Today's row shows the time in and time out from steps 3.3 and 3.5.                                |
| 3.9  | Open **My profile**.                                       | Juan's details are shown and nothing on the page can be edited.                                   |

## Flow 4: leave, approval, balance and attendance

| Step | Do this                                                                                           | Expect                                                                                           |
| ---- | ------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| 4.1  | Still as `juan`: My leave.                                                                        | Balances for this year are listed (Vacation Leave, Sick Leave, Service Incentive Leave).         |
| 4.2  | Write down the Vacation Leave days **Left**.                                                      |                                                                                                  |
| 4.3  | **File leave**: Vacation Leave, first and last day both next Monday, reason `Family event`.       | The request appears under Requests as waiting for HR, with a **Cancel** link.                    |
| 4.4  | Sign out, sign in as `admin`, open Leave.                                                         | Juan's request is under **Waiting for a decision**. Home also shows "1 leave request" to review. |
| 4.5  | Press **Approve**.                                                                                | The request moves to All requests as Approved.                                                   |
| 4.6  | Attendance, pick next Monday with the date box.                                                   | Juan's row shows **On leave**.                                                                   |
| 4.7  | Leave calendar, this month.                                                                       | Next Monday shows `Dela Cruz · VL`.                                                              |
| 4.8  | Sign out, sign in as `juan`, My leave.                                                            | The request is Approved and Vacation Leave **Left** is one day less than in 4.2.                 |
| 4.9  | As `admin`, Leave, All requests: **Cancel** the approved request. Then check 4.8 again as `juan`. | The day is back in the balance, and next Monday's attendance row no longer says On leave.        |

## Flow 5: a payroll from period to payslip

As `admin`. Use the cutoff that contains today for 5.1 to 5.6. Wagecraft refuses to post a
cutoff that hasn't ended, so run 5.7 to 5.9 on the day after the cutoff ends, or run the whole
flow on the last day of a cutoff.

| Step | Do this                                                                                                                  | Expect                                                                                                                         |
| ---- | ------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------ |
| 5.1  | Payroll, **New pay period**: this month, the cutoff that contains today, pay date five days after it, rule pack PH-2026. | The period appears as Draft. A panel lists unfinished work (pending leave, overtime, missing time-outs) or says there is none. |
| 5.2  | Open the period, press **Compute payroll**.                                                                              | The status becomes Computed and the register lists Juan with gross, contributions, tax and net pay.                            |
| 5.3  | Press **View** on Juan's row.                                                                                            | The payslip shows the earnings, SSS, PhilHealth, Pag-IBIG, the employer contributions and the net pay.                         |
| 5.4  | Press **Approve**.                                                                                                       | Status Approved. **Post payroll** and **Send back** appear.                                                                    |
| 5.5  | Press **Send back**, reason `Checking`, confirm.                                                                         | Status back to Computed.                                                                                                       |
| 5.6  | **Approve** again. Then **Download CSV** and **Download PDF**.                                                           | Status Approved. Both downloads save a file you can open; the PDF shows the company name.                                      |
| 5.7  | Press **Post payroll**, read the warning, press **Yes, post it**.                                                        | Status Posted. The page says attendance for these days is locked. Backups lists a new **Before posting payroll** backup.       |
| 5.8  | Attendance, today, press **Correct** on Juan's row and save a change.                                                    | Refused: "This day is in a posted payroll period, so it can't be changed."                                                     |
| 5.9  | Sign out, sign in as `juan`, My payslips.                                                                                | The period is listed. **View** shows the same figures as 5.3; **Download PDF** saves a PDF with the company name.              |

## Flow 6: back up, change, restore

As `admin`.

| Step | Do this                                                                          | Expect                                                                                                     |
| ---- | -------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| 6.1  | Backups, **Back up now**, pick the Desktop.                                      | A Manual backup appears in the list, and the Desktop has a file named `wagecraft-backup-<date>_<time>.db`. |
| 6.2  | Employees, open Juan, **Edit**, change the mobile number to `09170000000`, save. | The profile shows the new number.                                                                          |
| 6.3  | Backups, **Restore a backup…**, pick the Desktop file.                           | A panel shows the file name, when it was saved, how many employees it holds and the last posted payroll.   |
| 6.4  | Type `RESTORE` and confirm.                                                      | "Restoring… Wagecraft will restart in a moment." The app restarts on the Sign in screen.                   |
| 6.5  | Sign in as `admin`, open Juan's profile.                                         | The mobile number is the old one again.                                                                    |
| 6.6  | Backups.                                                                         | The list has a **Before a restore** backup taken just before 6.4.                                          |

## Flow 7: a Staff user can't reach HR screens or commands

This flow needs a development build (`pnpm tauri dev`), where right-clicking the page offers
**Inspect** and opens the developer console. Sign in as `juan` and open the console.

| Step | Do this                                                                                                     | Expect                                                                                                        |
| ---- | ----------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------- |
| 7.1  | In the console run `history.pushState({}, "", "/employees"); dispatchEvent(new PopStateEvent("popstate"))`. | The page says only Admin and HR can see the employee list. No employee data shows.                            |
| 7.2  | Do the same with `/users`, `/payroll`, `/backups` and `/settings`.                                          | Each page shows its "Only Admin…" line instead of data.                                                       |
| 7.3  | Run `const { call } = await import("/src/lib/ipc.ts")`, then `await call("auth_me")`.                       | Returns juan's own account, proving the console is talking to the running backend.                            |
| 7.4  | Run `await call("employee_get", { id: 1 })`.                                                                | Rejected with an error whose `code` is `FORBIDDEN`. Juan's record is not returned, even though it is his own. |
| 7.5  | Run `await call("payroll_compute", { periodId: 1 })`.                                                       | Rejected with `FORBIDDEN`. Payroll shows no new computation.                                                  |

## Results

| Release | Date | Tester | Flow 1 | Flow 2 | Flow 3 | Flow 4 | Flow 5 | Flow 6 | Flow 7 | Notes |
| ------- | ---- | ------ | ------ | ------ | ------ | ------ | ------ | ------ | ------ | ----- |
|         |      |        |        |        |        |        |        |        |        |       |

Mark each flow Pass or Fail. A Fail blocks the release until it is fixed and the flow is re-run.

# ADR-010: PDF templates in React; file writing in Rust

- **Status:** Accepted
- **Context:** Payslips need a clean printable layout, and the webview should not have broad file-system access.
- **Decision:** `@react-pdf/renderer` builds the PDF bytes in the UI; a Rust command checks that someone is signed in, opens the native Save dialog, and writes the file. Ownership is checked by the command that returns the payslip data (§6.6). CSV is generated entirely in Rust.
- **Alternatives:** _Rust PDF crates_ (`printpdf`, `typst`) are powerful but slower to design layouts with. _Browser print-to-PDF_ behaves inconsistently across webviews.
- **Consequences:** ➕ layouts are built with familiar React skills; frontend capabilities stay minimal. ➖ PDF bytes cross the IPC bridge (fine for payslip sizes).

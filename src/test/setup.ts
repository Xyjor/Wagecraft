import { configure } from "@testing-library/react";

// findBy* and waitFor give up after 1s by default. With the whole suite running in parallel,
// lazy imports and big pages sometimes take longer, and tests failed only under load.
configure({ asyncUtilTimeout: 5000 });

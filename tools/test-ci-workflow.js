#!/usr/bin/env node
// Contract test for .github/workflows/ci.yml.
//
// This is a smoke test over the raw file rather than a YAML schema check: its job
// is to catch a workflow that quietly stops running the build gate, the tests or
// the packaging step, which is the regression that matters here.
const { test } = require("node:test");
const assert = require("node:assert");
const fs = require("node:fs");
const path = require("node:path");

const repo = path.join(__dirname, "..");
const workflowPath = path.join(repo, ".github", "workflows", "ci.yml");

function workflow() {
  return fs.readFileSync(workflowPath, "utf8");
}

test("the repository has a workflow that builds, tests and packages", () => {
  assert.ok(fs.existsSync(workflowPath), ".github/workflows/ci.yml is missing");
  const text = workflow();
  assert.match(text, /bash tools\/build-web\.sh/, "the workflow must run the build gate");
  assert.match(text, /bash tools\/ci-test\.sh/, "the workflow must run the test entry point");
  assert.match(text, /bash tools\/package-site\.sh/, "the workflow must package the site");
});

test("the deployed site is the packaged directory", () => {
  const text = workflow();
  assert.match(text, /actions\/upload-pages-artifact/, "Pages needs the uploaded artifact");
  assert.match(text, /path:\s*_site/, "the uploaded path must be the packaged site");
  assert.match(text, /actions\/deploy-pages/, "Pages needs a deploy step");
});

test("deployment is limited to main and has the permissions it needs", () => {
  const text = workflow();
  assert.match(text, /branches:\s*\[\s*main\s*\]/, "pushes to other branches must not deploy");
  assert.match(text, /github\.ref == 'refs\/heads\/main'/, "the deploy job must be main-only");
  assert.match(text, /pages:\s*write/, "Pages deployment needs pages: write");
  assert.match(text, /id-token:\s*write/, "Pages deployment needs an OIDC token");
  assert.match(text, /workflow_dispatch/, "the pipeline must be runnable by hand");
});

test("the shared test entry point runs the repository-level tests", () => {
  // CI runs tools/ci-test.sh, so a test missing from it never runs in CI.
  const entry = fs.readFileSync(path.join(repo, "tools", "ci-test.sh"), "utf8");
  for (const file of ["tools/test-package.js", "tools/test-ci-workflow.js"]) {
    assert.match(entry, new RegExp(file.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")),
                 `${file} must run from tools/ci-test.sh`);
  }
  assert.match(entry, /cargo test -p collateral-sim/,
               "the ballistics tests must run from tools/ci-test.sh");
});

test("the browser test has Chromium available", () => {
  const text = workflow();
  assert.match(text, /playwright install/, "the browser test needs Chromium installed");
  assert.match(text, /setup-python/, "the browser test needs Python");
});

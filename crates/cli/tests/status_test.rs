mod utils;

use starbase_sandbox::predicates::prelude::*;
use utils::*;

mod status {
    use super::*;

    #[test]
    fn errors_when_nothing_configured() {
        let sandbox = create_empty_proto_sandbox();

        let assert = sandbox
            .run_bin(|cmd| {
                cmd.arg("status");
            })
            .failure();

        assert.stderr(predicate::str::contains("No tools have been configured"));
    }

    #[test]
    fn reports_all_non_global_configs() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file(".proto/.prototools", r#"go = "*""#);
        sandbox.create_file("a/.prototools", r#"node = "*""#);
        sandbox.create_file("a/b/.prototools", r#"npm = "*""#);
        sandbox.create_file("a/b/c/.prototools", r#"bun = "*""#);

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("status").current_dir(sandbox.path().join("a/b/c"));
        });

        let output = assert.output();

        assert!(predicate::str::contains("node").eval(&output));
        assert!(predicate::str::contains("npm").eval(&output));
        assert!(predicate::str::contains("bun").eval(&output));
        assert!(predicate::str::contains("go").not().eval(&output));
    }

    #[test]
    fn only_includes_local_config() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file(".proto/.prototools", r#"go = "*""#);
        sandbox.create_file("a/.prototools", r#"node = "*""#);
        sandbox.create_file("a/b/.prototools", r#"npm = "*""#);
        sandbox.create_file("a/b/c/.prototools", r#"bun = "*""#);

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("status")
                .arg("--config-mode")
                .arg("local")
                .current_dir(sandbox.path().join("a/b/c"));
        });

        let output = assert.output();

        assert!(predicate::str::contains("node").not().eval(&output));
        assert!(predicate::str::contains("npm").not().eval(&output));
        assert!(predicate::str::contains("bun").eval(&output));
        assert!(predicate::str::contains("go").not().eval(&output));
    }

    #[test]
    fn can_include_global_config() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file(".proto/.prototools", r#"go = "*""#);
        sandbox.create_file("a/.prototools", r#"node = "*""#);
        sandbox.create_file("a/b/.prototools", r#"npm = "*""#);
        sandbox.create_file("a/b/c/.prototools", r#"bun = "*""#);

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("status")
                .arg("--config-mode")
                .arg("upwards-global")
                .current_dir(sandbox.path().join("a/b/c"));
        });

        assert.debug();

        let output = assert.output();

        assert!(predicate::str::contains("node").eval(&output));
        assert!(predicate::str::contains("npm").eval(&output));
        assert!(predicate::str::contains("bun").eval(&output));
        assert!(predicate::str::contains("go").eval(&output));
    }

    #[test]
    fn can_include_ecosystem_config() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file(".prototools", r#"bun = "*""#);
        sandbox.create_file("package.json", r#"{ "engines": { "node": ">=20" } }"#);

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("status");
        });

        let output = assert.output();

        assert!(predicate::str::contains("node").eval(&output));
        assert!(predicate::str::contains(">=20").eval(&output));
    }

    #[test]
    fn global_doesnt_overwrite_local() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file(".proto/.prototools", r#"node = "18""#);
        sandbox.create_file("a/.prototools", r#"node = "20""#);

        let assert = sandbox.run_bin(|cmd| {
            cmd.arg("status")
                .arg("--config-mode")
                .arg("upwards-global")
                .current_dir(sandbox.path().join("a"));
        });

        let output = assert.output();

        assert!(predicate::str::contains("node").eval(&output));
        assert!(predicate::str::contains("20").eval(&output));
    }
}

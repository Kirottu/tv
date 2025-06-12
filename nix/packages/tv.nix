{
  inputs,
  lib,
  rustPlatform,
  cargo,
  rustc,
  ...
}:
let
  cargoToml = builtins.fromTOML (builtins.readFile ../../Cargo.toml);
  pname = cargoToml.package.name;
  version = cargoToml.package.version;
in
rustPlatform.buildRustPackage {
  inherit pname version;
  src = builtins.path {
    path = lib.sources.cleanSource inputs.self;
    name = "${pname}-${version}";
  };

  strictDeps = true;

  cargoLock = {
    lockFile = ../../Cargo.lock;
  };

  nativeBuildInputs = [
    rustc
    cargo
  ];

  buildInputs = [
  ];

  doCheck = true;
  checkInputs = [
    cargo
    rustc
  ];

  CARGO_BUILD_INCREMENTAL = "false";
  RUST_BACKTRACE = "full";

  meta = {
    description = "Small helper tool for managing desktop/TV state";
    homepage = "https://github.com/Kirottu/tv";
    mainProgram = pname;
    maintainers = with lib.maintainers; [ Kirottu ];
  };
}

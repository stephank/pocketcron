{
  outputs = { self }: {
    overlays.default = final: prev: {
      pocketcron = final.rustPlatform.buildRustPackage {
        name = "pocketcron";
        src = ./.;
        cargoLock.lockFile = ./Cargo.lock;
      };
    };
  };
}

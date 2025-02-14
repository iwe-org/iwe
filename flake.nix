{
  description = "IWE- Markdown writing assistant";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs";
    iwe = {
      url = "github:iwe-org/iwe";
      flake = false;
    };
  };

  outputs =
    {
      nixpkgs,
      iwe,
      ...
    }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
      version = iwe.lastModifiedDate;
      builder =
        { name, description }:
        pkgs.rustPlatform.buildRustPackage {
          pname = name;
          inherit version;
          src = iwe;
          cargoHash = "sha256-K8RxVYHh0pStQyHMiLLeUakAoK1IMoUtCNg70/NfDiI=sha256-3+DZi5yP9VYy4RwWDDZy9DM/fkBwP8rKAtY+C6aVAPw=";
          useFetchCargoVendor = true;
          buildPhase = ''
            cargo build --release --package ${name}
          '';

          installPhase = ''
            runHook preInstall
            mkdir -p $out/bin
            cp target/release/${name} $out/bin
            runHook postInstall
          '';

          meta = with pkgs.lib; {
            description = description;
            homepage = "https://iwe.md";
            license = licenses.asl20;
            mainProgram = name;
          };
        };
    in
    {
      packages.${system} = {
        iwe = builder {
          name = "iwe";
          description = "Markdown writing assistant and Personal Knowledge Management tool";
        };
        iwes = builder {
          name = "iwes";
          description = "Language Server for iwe";

        };
      };
    };
}

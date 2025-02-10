{
  description = "IWE- Markdown writing assistant";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs";
  };

  outputs =
    { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
      version = "0.0.17";
      commonAttrs = {
        src = pkgs.fetchFromGitHub {
          owner = "iwe-org";
          repo = "iwe";
          tag = "iwe-v${version}";
          hash = "sha256-eE84KzYJTJ39UDQt3VZpSIba/P+7VFR9K6+MSMlg0Wc=";
        };
        cargoHash = "sha256-K8RxVYHh0pStQyHMiLLeUakAoK1IMoUtCNg70/NfDiI=sha256-3+DZi5yP9VYy4RwWDDZy9DM/fkBwP8rKAtY+C6aVAPw=";
        useFetchCargoVendor = true;
      };
    in
    {
      packages.${system} = {
        iwe = pkgs.rustPlatform.buildRustPackage {
          pname = "iwe";
          version = "0.0.17";
          inherit (commonAttrs)
            src
            cargoHash
            useFetchCargoVendor
            ;
          buildPhase = ''
            cargo build --release --package iwe
          '';

          installPhase = ''
            runHook preInstall
            mkdir -p $out/bin
            cp target/release/iwe $out/bin
            runHook postInstall
          '';

          meta = with pkgs.lib; {
            description = "Markdown writing assistant and Personal Knowledge Management tool";
            homepage = "https://iwe.md";
            license = licenses.asl20;
            mainProgram = "iwe";
          };
        };
        iwes = pkgs.rustPlatform.buildRustPackage {
          pname = "iwes";
          version = "0.0.17";
          inherit (commonAttrs)
            src
            cargoHash
            useFetchCargoVendor
            ;
          buildPhase = ''
            cargo build --release --package iwes
          '';

          installPhase = ''
            runHook preInstall
            mkdir -p $out/bin
            cp target/release/iwes $out/bin
            runHook postInstall
          '';

          meta = with pkgs.lib; {
            description = "Language Server for iwe";
            homepage = "https://iwe.md";
            license = licenses.asl20;
            mainProgram = "iwes";
          };
        };
      };

      defaultPackages.${system} = self.packages.${system}.iwe;
    };
}

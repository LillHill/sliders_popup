{
  description = "Lightweight Wayland slider popup with JSON stdin config";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      supportedSystems = [ "x86_64-linux" "aarch64-linux" ];
      forAllSystems = nixpkgs.lib.genAttrs supportedSystems;
      pkgsFor = system: nixpkgs.legacyPackages.${system};
    in
    {
      packages = forAllSystems (system:
        let pkgs = pkgsFor system; in
        {
          default = self.packages.${system}.sliders_popup;

          sliders_popup = pkgs.stdenv.mkDerivation {
            pname = "sliders_popup";
            version = "0.1.0";
            src = self;

            nativeBuildInputs = with pkgs; [
              pkg-config
            ];

            buildInputs = with pkgs; [
              gtk3
              gtk-layer-shell
            ];

            makeFlags = [ "PREFIX=$(out)" ];

            meta = with pkgs.lib; {
              description = "Lightweight Wayland slider popup with JSON stdin config";
              license = licenses.mit;
              platforms = platforms.linux;
              mainProgram = "sliders_popup";
            };
          };
        }
      );

      devShells = forAllSystems (system:
        let pkgs = pkgsFor system; in
        {
          default = pkgs.mkShell {
            inputsFrom = [ self.packages.${system}.sliders_popup ];
          };
        }
      );
    };
}

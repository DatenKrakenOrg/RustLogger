{
  description = "RustLogger";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      pkgs = import nixpkgs {
        system = "x86_64-linux";
      };
    in
    {
      devShells.x86_64-linux.default = pkgs.mkShell {
        packages = with pkgs;[
          python313
          pre-commit
          rustc
          rustfmt
          cargo
          openssl
          lazygit
          pkg-config
          rustup
          dockerfmt
          nodejs
          lazydocker
        ];

        shellHook = ''
        '';
      };
    };
}

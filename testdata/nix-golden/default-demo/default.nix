{ lib, stdenv, fetchFromGitHub, pkg-config, openssl, zlib }:
stdenv.mkDerivation rec {
  pname = "demo";
  version = "1.2.3";
  homepage = "https://example.com/demo";

  nativeBuildInputs = [ pkg-config ];
  buildInputs = [ openssl zlib ];

  meta = with lib; {
    description = "Demo package";
    homepage = "https://example.com/demo";
    license = licenses.mit;
  };
}

Release: 1

%define openssldir /var/ssl

Summary: Secure Sockets Layer and cryptography libraries
Name: openssl
Version: 1.0.2e
Source0: ftp://ftp.openssl.org/source/%{name}-%{version}.tar.gz
License: OpenSSL
URL: http://www.openssl.org/
Packager: Damien Miller <djm@mindrot.org>
Provides: SSL
BuildRequires: perl, sed

%description
The OpenSSL Project is a collaborative effort to develop a robust,
commercial-grade, fully featured, and Open Source toolkit implementing the
Secure Sockets Layer (SSL v2/v3) and Transport Layer Security (TLS v1)
protocols as well as a full-strength general purpose cryptography library.

%package devel
Summary: OpenSSL development files
Requires: openssl

%description devel
This package contains the OpenSSL development files.

%prep
%setup -q

%build
./Configure --prefix=/usr --openssldir=%{openssldir}
make

%install
rm -rf $RPM_BUILD_ROOT
make INSTALL_PREFIX="$RPM_BUILD_ROOT" install

%files
%defattr(0644,root,root,0755)
%doc CHANGES LICENSE NEWS README

%files devel
%defattr(0644,root,root,0755)
%attr(0644,root,root) /usr/lib/*.a

%changelog
* Sun Jun 6 2005 Richard Levitte <richard@levitte.org>
- Initial package

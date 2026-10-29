#!/usr/bin/env python3
"""Generate lamina's signed fixtures (todo B3c) with pyHanko.

Usage (from the repository root):

    python3 -m venv .venv && .venv/bin/pip install -r fixtures/signed/requirements.txt
    .venv/bin/python fixtures/signed/make.py

Writes, in fixtures/signed/:

* approval.pdf: one approval signature in field `Sig1`, added as an incremental update;
* certified-p1.pdf, certified-p2.pdf, certified-p3.pdf: a certification signature with DocMDP
  `/P` 1 (no changes), 2 (form filling and signing) and 3 (also annotations), referenced from
  the catalog's `/Perms /DocMDP`;
* locked.pdf: an approval signature whose field has a FieldMDP `/Lock` (`/Action /All`);
* timestamp.pdf: a document timestamp (`/Type /DocTimeStamp`, ISO 32000-2 §12.8.5);
* signed-encrypted.pdf: an AES-128 file with an empty user password (owner password
  `owner-pw`), signed in an encrypted incremental update.

Every base is the same one-page document with an Info dictionary, written by pypdf. The keys
and certificates (a throwaway CA, a signer and a timestamping authority) are generated in
memory on every run and never written, so the files are not byte-reproducible; their
structure, which is what the tests read, is. Timestamps use a fixed time.
"""

from __future__ import annotations

import datetime
import io
from pathlib import Path

from asn1crypto import keys as asn1_keys
from asn1crypto import x509 as asn1_x509
from cryptography import x509
from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.asymmetric import rsa
from cryptography.x509.oid import ExtendedKeyUsageOID, NameOID
from pyhanko.pdf_utils.incremental_writer import IncrementalPdfFileWriter
from pyhanko.sign import fields, signers, timestamps
from pyhanko.sign.fields import FieldMDPAction, FieldMDPSpec, MDPPerm, SigFieldSpec
from pyhanko_certvalidator.registry import SimpleCertificateStore
from pypdf import PdfWriter

HERE = Path(__file__).resolve().parent
FIXED = datetime.datetime(2026, 10, 29, 12, 0, 0, tzinfo=datetime.timezone.utc)
OWNER = "owner-pw"


def key() -> rsa.RSAPrivateKey:
    return rsa.generate_private_key(public_exponent=65537, key_size=2048)


def cert(subject: str, public, issuer: str, issuer_key, *, ca: bool = False, tsa: bool = False):
    builder = (
        x509.CertificateBuilder()
        .subject_name(x509.Name([x509.NameAttribute(NameOID.COMMON_NAME, subject)]))
        .issuer_name(x509.Name([x509.NameAttribute(NameOID.COMMON_NAME, issuer)]))
        .public_key(public)
        .serial_number(x509.random_serial_number())
        .not_valid_before(FIXED - datetime.timedelta(days=1))
        .not_valid_after(FIXED + datetime.timedelta(days=3650))
        .add_extension(x509.BasicConstraints(ca=ca, path_length=None), critical=True)
    )
    if tsa:
        builder = builder.add_extension(x509.ExtendedKeyUsage([ExtendedKeyUsageOID.TIME_STAMPING]), critical=True)
    return builder.sign(issuer_key, hashes.SHA256())


def asn1_cert(c) -> asn1_x509.Certificate:
    return asn1_x509.Certificate.load(c.public_bytes(serialization.Encoding.DER))


def asn1_key(k) -> asn1_keys.PrivateKeyInfo:
    der = k.private_bytes(
        serialization.Encoding.DER, serialization.PrivateFormat.PKCS8, serialization.NoEncryption()
    )
    return asn1_keys.PrivateKeyInfo.load(der)


def base(*, encrypt: bool = False) -> bytes:
    w = PdfWriter()
    w.add_blank_page(612, 792)
    w.add_metadata({"/Title": "Signed fixture", "/Producer": "lamina fixtures/signed/make.py"})
    if encrypt:
        w.encrypt(user_password="", owner_password=OWNER, algorithm="AES-128")
    out = io.BytesIO()
    w.write(out)
    return out.getvalue()


def main() -> None:
    ca_key, signer_key, tsa_key = key(), key(), key()
    ca = cert("lamina test CA", ca_key.public_key(), "lamina test CA", ca_key, ca=True)
    signer_cert = cert("lamina test signer", signer_key.public_key(), "lamina test CA", ca_key)
    tsa_cert = cert("lamina test TSA", tsa_key.public_key(), "lamina test CA", ca_key, tsa=True)
    store = SimpleCertificateStore.from_certs([asn1_cert(ca)])
    signer = signers.SimpleSigner(
        signing_cert=asn1_cert(signer_cert), signing_key=asn1_key(signer_key), cert_registry=store
    )
    tsa = timestamps.DummyTimeStamper(
        tsa_cert=asn1_cert(tsa_cert),
        tsa_key=asn1_key(tsa_key),
        certs_to_embed=SimpleCertificateStore.from_certs([asn1_cert(ca)]),
        fixed_dt=FIXED,
    )

    def sign(name: str, meta: signers.PdfSignatureMetadata, *, spec: SigFieldSpec | None = None, encrypt=False):
        w = IncrementalPdfFileWriter(io.BytesIO(base(encrypt=encrypt)))
        if encrypt:
            w.encrypt("")
        fields.append_signature_field(w, spec or SigFieldSpec(sig_field_name="Sig1", box=(50, 700, 250, 750)))
        out = io.BytesIO()
        signers.PdfSigner(meta, signer=signer).sign_pdf(w, output=out)
        (HERE / name).write_bytes(out.getvalue())

    def meta(**kw) -> signers.PdfSignatureMetadata:
        return signers.PdfSignatureMetadata(
            field_name="Sig1", subfilter=fields.SigSeedSubFilter.PADES, md_algorithm="sha256", **kw
        )

    sign("approval.pdf", meta())
    for p, perm in [(1, MDPPerm.NO_CHANGES), (2, MDPPerm.FILL_FORMS), (3, MDPPerm.ANNOTATE)]:
        sign(f"certified-p{p}.pdf", meta(certify=True, docmdp_permissions=perm))
    sign(
        "locked.pdf",
        meta(),
        spec=SigFieldSpec(
            sig_field_name="Sig1", box=(50, 700, 250, 750), field_mdp_spec=FieldMDPSpec(FieldMDPAction.ALL)
        ),
    )
    sign("signed-encrypted.pdf", meta(), encrypt=True)

    w = IncrementalPdfFileWriter(io.BytesIO(base()))
    out = io.BytesIO()
    signers.PdfTimeStamper(tsa).timestamp_pdf(w, md_algorithm="sha256", output=out)
    (HERE / "timestamp.pdf").write_bytes(out.getvalue())


if __name__ == "__main__":
    main()

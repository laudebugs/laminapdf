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
* two-signatures.pdf: two approval signatures, in fields `Sig1` and `Sig2`, each added in its
  own incremental update (3 revisions);
* timestamp.pdf: a document timestamp (`/Type /DocTimeStamp`, ISO 32000-2 §12.8.5);
* approval-organised.pdf: three pages with an outline (one item per page), named destinations
  (`page-1` to `page-3`), a text field `Name` whose widget is on page 1, and tags (a
  `/StructTreeRoot` with one `/P` per page), then an approval signature whose visible widget is
  on page 2 (E2 plan §9);
* certified-p2-outline.pdf: the same three pages with the outline and named destinations,
  certified at P2;
* locked-page1.pdf: the same three pages and text field, then an approval signature on page 3
  whose FieldMDP lock (`/Action /Include`) covers `Name`, whose widget is on page 1;
* usage-rights.pdf: the same three pages with a text annotation on page 1, and `/Perms /UR3`
  granting `/Annots [/Create /Delete]` and `/Form [/FillIn]`. Only Adobe's key can sign usage
  rights, so the dictionary is written with no valid signature: the planner reads its
  structure, not its validity;
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
from pypdf.generic import (
    ArrayObject,
    BooleanObject,
    DecodedStreamObject,
    DictionaryObject,
    FloatObject,
    NameObject,
    NumberObject,
    TextStringObject,
)

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


def name(n: str) -> NameObject:
    return NameObject(n)


def three_pages(w: PdfWriter, *, tagged: bool) -> list:
    """Three Letter pages, each drawing "Page n"; with `tagged`, each in a marked-content
    sequence (MCID 0) with `/StructParents n`. Returns the pages' `/P` elements (or none)."""
    font = w._add_object(DictionaryObject({
        name("/Type"): name("/Font"), name("/Subtype"): name("/Type1"), name("/BaseFont"): name("/Helvetica"),
    }))
    elements = []
    for n in range(3):
        page = w.add_blank_page(612, 792)
        body = f"BT /F1 24 Tf 72 700 Td (Page {n + 1}) Tj ET".encode()
        if tagged:
            body = b"/P <</MCID 0>> BDC " + body + b" EMC"
            page[name("/StructParents")] = NumberObject(n)
            elements.append(w._add_object(DictionaryObject({
                name("/Type"): name("/StructElem"), name("/S"): name("/P"),
                name("/Pg"): page.indirect_reference, name("/K"): NumberObject(0),
            })))
        stream = DecodedStreamObject()
        stream.set_data(body)
        page[name("/Contents")] = w._add_object(stream)
        page[name("/Resources")] = DictionaryObject({name("/Font"): DictionaryObject({name("/F1"): font})})
    return elements


def add_outline(w: PdfWriter) -> None:
    """A bookmark per page."""
    for n in range(3):
        w.add_outline_item(f"Page {n + 1}", n)


def add_named_destinations(w: PdfWriter) -> None:
    """`page-1` to `page-3` in the `/Names /Dests` tree, each fitting its page."""
    names = ArrayObject()
    for n, page in enumerate(w.pages):
        names.extend([TextStringObject(f"page-{n + 1}"), ArrayObject([page.indirect_reference, name("/Fit")])])
    w._root_object[name("/Names")] = DictionaryObject({
        name("/Dests"): w._add_object(DictionaryObject({name("/Names"): names})),
    })


def add_annotation(w: PdfWriter, annot: DictionaryObject) -> None:
    """`annot` on page 1."""
    page = w.pages[0]
    annot[name("/P")] = page.indirect_reference
    page[name("/Annots")] = ArrayObject([w._add_object(annot)])


def rect(*values: float) -> ArrayObject:
    return ArrayObject([FloatObject(v) for v in values])


def add_name_field(w: PdfWriter) -> None:
    """A text field `Name` holding "Ada", its widget on page 1."""
    add_annotation(w, DictionaryObject({
        name("/Type"): name("/Annot"), name("/Subtype"): name("/Widget"), name("/FT"): name("/Tx"),
        name("/T"): TextStringObject("Name"), name("/V"): TextStringObject("Ada"), name("/F"): NumberObject(4),
        name("/Rect"): rect(72, 600, 272, 624),
    }))
    field = w.pages[0]["/Annots"][0]
    w._root_object[name("/AcroForm")] = DictionaryObject({name("/Fields"): ArrayObject([field])})


def add_note(w: PdfWriter) -> None:
    """A text note on page 1."""
    add_annotation(w, DictionaryObject({
        name("/Type"): name("/Annot"), name("/Subtype"): name("/Text"), name("/Rect"): rect(300, 700, 320, 720),
        name("/Contents"): TextStringObject("A note"),
    }))


def add_tags(w: PdfWriter, elements: list) -> None:
    """A structure tree: a `/Document` holding the pages' `/P` elements, with its parent tree."""
    tree = w._add_object(DictionaryObject({name("/Type"): name("/StructTreeRoot")}))
    doc = w._add_object(DictionaryObject({
        name("/Type"): name("/StructElem"), name("/S"): name("/Document"), name("/P"): tree,
        name("/K"): ArrayObject(elements),
    }))
    parents = ArrayObject()
    for n, e in enumerate(elements):
        e.get_object()[name("/P")] = doc
        parents.extend([NumberObject(n), ArrayObject([e])])
    tree.get_object().update({
        name("/K"): doc,
        name("/ParentTree"): DictionaryObject({name("/Nums"): parents}),
        name("/ParentTreeNextKey"): NumberObject(len(elements)),
    })
    w._root_object[name("/StructTreeRoot")] = tree
    w._root_object[name("/MarkInfo")] = DictionaryObject({name("/Marked"): BooleanObject(True)})


def add_usage_rights(w: PdfWriter) -> None:
    """`/Perms /UR3` granting `/Annots [/Create /Delete]` and `/Form [/FillIn]`. Only Adobe's key
    signs usage rights, so the signature is a placeholder: the planner reads the structure."""
    rights = DictionaryObject({
        name("/Type"): name("/TransformParams"), name("/V"): name("/2.2"),
        name("/Annots"): ArrayObject([name("/Create"), name("/Delete")]),
        name("/Form"): ArrayObject([name("/FillIn")]),
    })
    sig = w._add_object(DictionaryObject({
        name("/Type"): name("/Sig"), name("/Filter"): name("/Adobe.PPKLite"),
        name("/SubFilter"): name("/adbe.pkcs7.detached"), name("/ByteRange"): ArrayObject([NumberObject(0)] * 4),
        name("/Contents"): TextStringObject(""),
        name("/Reference"): ArrayObject([DictionaryObject({
            name("/Type"): name("/SigRef"), name("/TransformMethod"): name("/UR3"), name("/TransformParams"): rights,
        })]),
    }))
    w._root_object[name("/Perms")] = DictionaryObject({name("/UR3"): sig})


def written(w: PdfWriter) -> bytes:
    w.add_metadata({"/Title": "Signed fixture", "/Producer": "lamina fixtures/signed/make.py"})
    out = io.BytesIO()
    w.write(out)
    return out.getvalue()


def organised() -> bytes:
    """Three pages with an outline, named destinations, the `Name` field and tags."""
    w = PdfWriter()
    elements = three_pages(w, tagged=True)
    add_outline(w)
    add_named_destinations(w)
    add_name_field(w)
    add_tags(w, elements)
    return written(w)


def with_outline() -> bytes:
    """Three pages with an outline and named destinations."""
    w = PdfWriter()
    three_pages(w, tagged=False)
    add_outline(w)
    add_named_destinations(w)
    return written(w)


def with_field() -> bytes:
    """Three pages with the `Name` field."""
    w = PdfWriter()
    three_pages(w, tagged=False)
    add_name_field(w)
    return written(w)


def with_usage_rights() -> bytes:
    """Three pages with a note on page 1, and usage rights."""
    w = PdfWriter()
    three_pages(w, tagged=False)
    add_note(w)
    add_usage_rights(w)
    return written(w)


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

    def signed(pdf: bytes, meta: signers.PdfSignatureMetadata, spec: SigFieldSpec, *, encrypt=False) -> bytes:
        w = IncrementalPdfFileWriter(io.BytesIO(pdf))
        if encrypt:
            w.encrypt("")
        fields.append_signature_field(w, spec)
        out = io.BytesIO()
        signers.PdfSigner(meta, signer=signer).sign_pdf(w, output=out)
        return out.getvalue()

    def sign(name: str, meta: signers.PdfSignatureMetadata, *, spec: SigFieldSpec | None = None, encrypt=False):
        spec = spec or SigFieldSpec(sig_field_name="Sig1", box=(50, 700, 250, 750))
        (HERE / name).write_bytes(signed(base(encrypt=encrypt), meta, spec, encrypt=encrypt))

    def meta(field: str = "Sig1", **kw) -> signers.PdfSignatureMetadata:
        return signers.PdfSignatureMetadata(
            field_name=field, subfilter=fields.SigSeedSubFilter.PADES, md_algorithm="sha256", **kw
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

    page2 = SigFieldSpec(sig_field_name="Sig1", on_page=1, box=(50, 50, 250, 100))
    (HERE / "approval-organised.pdf").write_bytes(
        signed(organised(), meta(), page2)
    )
    (HERE / "certified-p2-outline.pdf").write_bytes(
        signed(with_outline(), meta(certify=True, docmdp_permissions=MDPPerm.FILL_FORMS), page2)
    )
    lock = FieldMDPSpec(FieldMDPAction.INCLUDE, fields=["Name"])
    page3 = SigFieldSpec(sig_field_name="Sig1", on_page=2, box=(50, 50, 250, 100), field_mdp_spec=lock)
    (HERE / "locked-page1.pdf").write_bytes(signed(with_field(), meta(), page3))
    (HERE / "usage-rights.pdf").write_bytes(with_usage_rights())

    once = signed(base(), meta(), SigFieldSpec(sig_field_name="Sig1", box=(50, 700, 250, 750)))
    twice = signed(once, meta("Sig2"), SigFieldSpec(sig_field_name="Sig2", box=(300, 700, 500, 750)))
    (HERE / "two-signatures.pdf").write_bytes(twice)

    w = IncrementalPdfFileWriter(io.BytesIO(base()))
    out = io.BytesIO()
    signers.PdfTimeStamper(tsa).timestamp_pdf(w, md_algorithm="sha256", output=out)
    (HERE / "timestamp.pdf").write_bytes(out.getvalue())


if __name__ == "__main__":
    main()

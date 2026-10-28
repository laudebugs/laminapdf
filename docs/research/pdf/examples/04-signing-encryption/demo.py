import sys, re, hashlib
from pypdf import PdfWriter
from pyhanko.sign import signers, fields
from pyhanko.pdf_utils.incremental_writer import IncrementalPdfFileWriter
from pyhanko.sign.fields import SigFieldSpec

w = PdfWriter(); w.add_blank_page(612, 792)
with open("in.pdf","wb") as f: w.write(f)

signer = signers.SimpleSigner.load("key.pem","cert.pem")
with open("in.pdf","rb") as inf:
    iw = IncrementalPdfFileWriter(inf)
    fields.append_signature_field(iw, SigFieldSpec(sig_field_name="Sig1", box=(50,700,250,750)))
    meta = signers.PdfSignatureMetadata(field_name="Sig1", subfilter=fields.SigSeedSubFilter.PADES, md_algorithm="sha256")
    with open("signed.pdf","wb") as out:
        signers.PdfSigner(meta, signer=signer).sign_pdf(iw, output=out)

data = open("signed.pdf","rb").read()
print("file size", len(data))
m = re.search(rb"/ByteRange\s*\[\s*(\d+)\s+(\d+)\s+(\d+)\s+(\d+)\s*\]", data)
a,b,c,d = map(int, m.groups()); print("ByteRange", a,b,c,d)
print("gap bytes", c-b, "hex placeholder chars", c-b-2)
print("c+d == filesize?", c+d == len(data))
print("byte at b:", data[b:b+1], "byte at c-1:", data[c-1:c])
# show sig dict
start = data.rfind(b"<<", 0, m.start())
i = data.find(b"/Type /Sig", max(0,m.start()-2000))
print(data[m.start()-400:m.end()+60].decode('latin1')[:900])
contents = data[b+1:c-1]
der = bytes.fromhex(contents.decode()).rstrip(b"\x00")
print("CMS DER length", len(der), "padding hex chars", (c-b-2) - 2*len(der))
h = hashlib.sha256(data[a:a+b] + data[c:c+d]).hexdigest(); print("sha256 over ranges", h)
open("cms.der","wb").write(bytes.fromhex(contents.decode()))

import sys
d=open(sys.argv[1],'rb').read()
b=d.replace(b'0000000015 00000 n',b'0000000099 00000 n').replace(b'startxref\n416',b'startxref\n400')
open(sys.argv[2],'wb').write(b)
c=b"BT\n/F1 24 Tf\n72 720 Td\n(Hello, Update) Tj\nET"
upd=bytearray(); off5=len(d)
upd+=b"5 0 obj\n<< /Length %d >>\nstream\n"%len(c)+c+b"\nendstream\nendobj\n"
x=len(d)+len(upd)
upd+=b"xref\n0 1\n0000000000 65535 f\r\n5 1\n%010d 00000 n\r\n"%off5
upd+=b"trailer\n<< /Size 6 /Root 1 0 R /Prev 416 >>\nstartxref\n%d\n%%%%EOF\n"%x
open(sys.argv[3],'wb').write(d+upd); print(upd.decode())

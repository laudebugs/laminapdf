# Minimal hand-written PDF exercising the imaging model (chapter 02 example).
import math
K = 4*(math.sqrt(2)-1)/3  # 0.5522847498
def circle(cx, cy, r):
    k = K*r
    return "\n".join([
        f"{cx+r:.3f} {cy:.3f} m",
        f"{cx+r:.3f} {cy+k:.3f} {cx+k:.3f} {cy+r:.3f} {cx:.3f} {cy+r:.3f} c",
        f"{cx-k:.3f} {cy+r:.3f} {cx-r:.3f} {cy+k:.3f} {cx-r:.3f} {cy:.3f} c",
        f"{cx-r:.3f} {cy-k:.3f} {cx-k:.3f} {cy-r:.3f} {cx:.3f} {cy-r:.3f} c",
        f"{cx+k:.3f} {cy-r:.3f} {cx+r:.3f} {cy-k:.3f} {cx+r:.3f} {cy:.3f} c",
        "h"])
c, s = math.cos(math.radians(30)), math.sin(math.radians(30))
content = f"""% 1. filled rectangle (DeviceRGB)
q
0.20 0.40 0.80 rg
72 600 200 120 re
f
Q
% 2. stroked Bezier circle, round joins, dashed
q
0.85 0.10 0.10 RG
6 w
1 j
[18 6] 0 d
{circle(430, 660, 70)}
S
Q
% 3. axial gradient clipped to a rectangle
q
72 380 468 150 re
W n
/Sh1 sh
Q
% 4. rotated square, 30 deg about its centre (306,220), 50% opacity
q
/GS1 gs
1 0 0 1 306 220 cm
{c:.6f} {s:.6f} {-s:.6f} {c:.6f} 0 0 cm
0 0.6 0.3 rg
0 0 0 RG
2 w
-60 -60 120 120 re
B
Q
"""
objs = {}
objs[1] = "<< /Type /Catalog /Pages 2 0 R >>"
objs[2] = "<< /Type /Pages /Kids [3 0 R] /Count 1 >>"
objs[3] = ("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R "
           "/Resources << /Shading << /Sh1 5 0 R >> /ExtGState << /GS1 7 0 R >> >> >>")
data = content.encode("latin-1")
objs[4] = f"<< /Length {len(data)} >>\nstream\n".encode() + data + b"\nendstream"
objs[5] = ("<< /ShadingType 2 /ColorSpace /DeviceRGB /Coords [72 0 540 0] "
           "/Function 6 0 R /Extend [true true] >>")
objs[6] = "<< /FunctionType 2 /Domain [0 1] /C0 [1 0.85 0.1] /C1 [0.5 0 0.6] /N 1 >>"
objs[7] = "<< /Type /ExtGState /ca 0.5 /CA 0.5 >>"
out = bytearray(b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n")
offs = {}
for i in sorted(objs):
    offs[i] = len(out)
    body = objs[i] if isinstance(objs[i], bytes) else objs[i].encode()
    out += f"{i} 0 obj\n".encode() + body + b"\nendobj\n"
xref = len(out)
n = max(objs)+1
out += f"xref\n0 {n}\n0000000000 65535 f \n".encode()
for i in range(1, n):
    out += f"{offs[i]:010d} 00000 n \n".encode()
out += f"trailer\n<< /Size {n} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n".encode()
open("imaging-demo.pdf", "wb").write(out)
print(content)

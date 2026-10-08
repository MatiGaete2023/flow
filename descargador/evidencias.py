"""PDF local de una captura PNG de Chrome. No persiste la imagen intermedia."""
import base64
import hashlib
import io
import warnings

from PIL import Image
from reportlab.lib.utils import ImageReader
from reportlab.pdfgen import canvas

import motor as m


def screenshot_pdf(result, digest, label):
    try:
        if result['digest'] != digest or len(result['png'])>40*1024*1024:
            raise ValueError()
        png=base64.b64decode(result['png'],validate=True)
        if not png.startswith(b'\x89PNG\r\n\x1a\n'):
            raise ValueError()
        with warnings.catch_warnings():
            warnings.simplefilter('error',Image.DecompressionBombWarning)
            with Image.open(io.BytesIO(png)) as image:
                if image.format!='PNG' or image.width<400 or image.height<200 or image.width*image.height>30_000_000:
                    raise ValueError()
                image.load()
                rgb=image.convert('RGB')
        # Tamaño proporcional a la captura: no recorta ni reduce a una hoja A4.
        width,height=rgb.size;width/=2;height/=2
        out=io.BytesIO();pdf=canvas.Canvas(out,pagesize=(width,height),pageCompression=1)
        pdf.setTitle(label)
        pdf.drawImage(ImageReader(rgb),0,0,width=width,height=height)
        pdf.showPage();pdf.save();data=out.getvalue();rgb.close()
        if not data.startswith(b'%PDF-') or not data.rstrip().endswith(b'%%EOF'):
            raise ValueError()
        return data,hashlib.sha256(data).hexdigest()
    except (KeyError,TypeError,ValueError,OSError,Image.DecompressionBombError,Image.DecompressionBombWarning):
        raise m.PocError('Chrome no entregó una captura PNG válida; no se guardó un PDF de esta consulta.') from None

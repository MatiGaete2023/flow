"""Native Windows capture without running another GUI scenario on import."""
from pathlib import Path


def capture_window(app,name):
    import ctypes
    import win32gui,win32ui,win32con
    from PIL import Image
    folder=Path('artifacts');folder.mkdir(exist_ok=True)
    app.update_idletasks()
    hwnd=win32gui.GetAncestor(app.winfo_id(),2)
    left,top,right,bottom=win32gui.GetWindowRect(hwnd)
    handle=win32gui.GetWindowDC(hwnd)
    source=win32ui.CreateDCFromHandle(handle);target=source.CreateCompatibleDC()
    bitmap=win32ui.CreateBitmap();bitmap.CreateCompatibleBitmap(source,right-left,bottom-top)
    target.SelectObject(bitmap)
    try:
        # PrintWindow also renders areas outside the runner's virtual monitor.
        printed=ctypes.windll.user32.PrintWindow(hwnd,target.GetSafeHdc(),2)
        if not printed:target.BitBlt((0,0),(right-left,bottom-top),source,(0,0),win32con.SRCCOPY)
        bmp=folder/(name+'.bmp');bitmap.SaveBitmapFile(target,str(bmp))
        with Image.open(bmp) as image:image.save(folder/(name+'.png'))
        bmp.unlink();print('Captura Windows: '+name+'.png')
    finally:
        target.DeleteDC();source.DeleteDC();win32gui.ReleaseDC(hwnd,handle);win32gui.DeleteObject(bitmap.GetHandle())

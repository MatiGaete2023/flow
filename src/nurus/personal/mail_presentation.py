"""Sober Outlook HTML made from literal reviewed text; no external resources."""
from html import escape
import re


def body_html(text):
    paragraphs = str(text).replace('\r\n', '\n').replace('\r', '\n').split('\n\n')
    content = ''.join('<p style="margin:0 0 12px;line-height:1.45">' + escape(p).replace('\n', '<br>') + '</p>' for p in paragraphs)
    return ('<div style="font-family:Calibri,Arial,sans-serif;font-size:11pt;color:#202020">' + content + '</div>')


def insert_before_signature(text, signature, styled=True):
    addition = (body_html(text) if styled else '<div>'+escape(text).replace('\n','<br>')+'</div>') + '<br>'
    match = re.search(r'<body\b[^>]*>', signature, flags=re.IGNORECASE)
    return signature[:match.end()] + addition + signature[match.end():] if match else addition + signature

# Entorno de verificación visual

- Linux: Xvfb, Mesa software, binario debug del mismo código. Teclas Tab enviadas
  a la única ventana devuelta para el PID de la demostración; se acredita foco de egui.
- Windows: binario release (huella comparada con el candidato), servidor de GitHub
  sin aceleración gráfica. Su OpenGL 1.1 de fábrica no inicia egui, por lo que las
  capturas usan Mesa LLVMpipe en una carpeta de pruebas aislada.
- Las DLL de Mesa no se incorporan al producto, no se instalan globalmente y no
  modifican archivos del sistema. Se verifica el SHA-256 del archivo antes de extraerlo.

Runtime de prueba fijado: mesa-dist-win 26.2.7 release MinGW x64.
SHA-256: 19c74aad7b09e682473b8de4c35b47e6498de97b71cae5511a231b8eb02f2b22.
El paquete MSVC de esa versión no incluye OpenGL por un fallo de compilación
documentado en la publicación del mantenedor. El workaround oficial es MinGW x64.

Fuentes de la dependencia de pruebas:

- [Publicación y huella del mantenedor](https://github.com/pal1000/mesa-dist-win/releases/tag/26.2.7).
- [Licencia MIT y distribución](https://github.com/pal1000/mesa-dist-win).
- [Despliegue por aplicación documentado por Mesa](https://docs.mesa3d.org/drivers/llvmpipe.html).

Esto prueba la interfaz sobre ambos sistemas y el renderizado por software del
servidor. No representa una prueba del controlador gráfico del equipo del usuario
ni de su sesión real de Outlook o SITFA.


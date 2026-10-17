"""Exercise SMTP STARTTLS and upload against a local disposable server."""
import socket,ssl,threading,subprocess,tempfile,pathlib
with tempfile.TemporaryDirectory(prefix='stupid-mail-smtp-test-') as temporary:
 root=pathlib.Path(temporary)
 (root/'tls.conf').write_text('[req]\nprompt=no\ndistinguished_name=dn\nx509_extensions=v3\n[dn]\nCN=localhost\n[v3]\nsubjectAltName=DNS:localhost\nbasicConstraints=critical,CA:TRUE\n')
 subprocess.run(['/usr/bin/openssl','req','-x509','-newkey','rsa:2048','-nodes','-days','1','-keyout',str(root/'key.pem'),'-out',str(root/'cert.pem'),'-config',str(root/'tls.conf')],check=True,capture_output=True)
 context=ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER);context.load_cert_chain(str(root/'cert.pem'),str(root/'key.pem'))
 server=socket.socket();server.bind(('127.0.0.1',0));server.listen();port=server.getsockname()[1];received=[];commands=[]
 def serve():
  conn,_=server.accept();f=conn.makefile('rwb',buffering=0);f.write(b'220 localhost test mail\r\n');auth=False
  while True:
   line=f.readline()
   if not line:break
   if auth:f.write(b'235 Authentication successful\r\n');auth=False;continue
   command=line.decode().strip();commands.append(command.split(' ',1)[0])
   if command.startswith('EHLO'):f.write(b'250-localhost\r\n250-STARTTLS\r\n250 AUTH PLAIN\r\n')
   elif command=='STARTTLS':
    f.write(b'220 Begin TLS\r\n');f.close();conn=context.wrap_socket(conn,server_side=True);f=conn.makefile('rwb',buffering=0)
   elif command.startswith('AUTH PLAIN'):
    if len(command.split())>2:f.write(b'235 Authentication successful\r\n')
    else:f.write(b'334 \r\n');auth=True
   elif command.startswith(('MAIL FROM','RCPT TO')):f.write(b'250 OK\r\n')
   elif command=='DATA':
    f.write(b'354 Send message\r\n')
    while True:
     part=f.readline()
     if part==b'.\r\n':break
     received.append(part)
    f.write(b'250 Accepted\r\n')
   elif command=='QUIT':f.write(b'221 Bye\r\n');break
   else:f.write(b'500 Unexpected\r\n')
  conn.close();server.close()
 thread=threading.Thread(target=serve,daemon=True);thread.start()
 message=b'From: fixture@icloud.com\r\nTo: fixture@example.com\r\nSubject: Local test only\r\n\r\nHello\r\n';(root/'mail.eml').write_bytes(message)
 cfg=f'silent\nshow-error\nssl-reqd\nmax-time=15\nurl="smtp://localhost:{port}"\ncacert="{root}/cert.pem"\nuser="fixture:abcd-efgh-ijkl-mnop"\nmail-from="fixture@icloud.com"\nmail-rcpt="fixture@example.com"\nupload-file="{root}/mail.eml"\n'
 result=subprocess.run(['/usr/bin/curl','--disable','--config','-'],input=cfg.encode(),capture_output=True,timeout=20)
 assert result.returncode==0,result.stderr.decode()
 assert 'STARTTLS' in commands and 'AUTH' in commands and b''.join(received)==message
 print('Passed SMTP STARTTLS, authentication, recipient, and MIME upload checks against local fixtures.')

# The interface calls these through the JavaScript bridge; the annotation must survive for WebView to expose them.
-keepattributes RuntimeVisibleAnnotations
-keep class com.emil.stupidmail.Bridge { @android.webkit.JavascriptInterface public <methods>; }

/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use canvas_traits::webgl::{webgl_channel, WebGLCommand, WebGLError, WebGLVersion};
use dom::bindings::codegen::Bindings::OESVertexArrayObjectBinding::{self, OESVertexArrayObjectMethods};
use dom::bindings::codegen::Bindings::OESVertexArrayObjectBinding::OESVertexArrayObjectConstants;
use dom::bindings::reflector::{DomObject, Reflector, reflect_dom_object};
use dom::bindings::root::{Dom, DomRoot, MutNullableDom};
use dom::webglrenderingcontext::WebGLRenderingContext;
use dom::webglvertexarrayobjectoes::WebGLVertexArrayObjectOES;
use dom_struct::dom_struct;
use js::conversions::ToJSValConvertible;
use js::jsapi::JSContext;
use js::jsval::{JSVal, NullValue};
use super::{WebGLExtension, WebGLExtensions, WebGLExtensionSpec};

#[dom_struct]
pub struct OESVertexArrayObject {
    reflector_: Reflector,
    ctx: Dom<WebGLRenderingContext>,
    bound_vao: MutNullableDom<WebGLVertexArrayObjectOES>,
}

impl OESVertexArrayObject {
    fn new_inherited(ctx: &WebGLRenderingContext) -> OESVertexArrayObject {
        Self {
            reflector_: Reflector::new(),
            ctx: Dom::from_ref(ctx),
            bound_vao: MutNullableDom::new(None)
        }
    }

    #[allow(unsafe_code)]
    fn get_current_binding(&self, cx:*mut JSContext) -> JSVal {
        rooted!(in(cx) let mut rval = NullValue());
        if let Some(bound_vao) = self.bound_vao.get() {
            unsafe {
                bound_vao.to_jsval(cx, rval.handle_mut());
            }
        }
        rval.get()
    }
}

impl OESVertexArrayObjectMethods for OESVertexArrayObject {
    // https://www.khronos.org/registry/webgl/extensions/OES_vertex_array_object/
    fn CreateVertexArrayOES(&self) -> Option<DomRoot<WebGLVertexArrayObjectOES>> {
        let (sender, receiver) = webgl_channel().unwrap();
        self.ctx.send_command(WebGLCommand::CreateVertexArray(sender));

        let result = receiver.recv().unwrap();
        result.map(|vao_id| WebGLVertexArrayObjectOES::new(&self.global(), vao_id))
    }

    // https://www.khronos.org/registry/webgl/extensions/OES_vertex_array_object/
    fn DeleteVertexArrayOES(&self, vao: Option<&WebGLVertexArrayObjectOES>) {
        if let Some(vao) = vao {
            if vao.is_deleted() {
                return;
            }

            // Unbind deleted VAO if currently bound
            if let Some(bound_vao) = self.bound_vao.get() {
                if bound_vao.id() == vao.id() {
                    self.bound_vao.set(None);
                    self.ctx.send_command(WebGLCommand::BindVertexArray(None));
                }
            }

            // Remove VAO references from buffers
            for (_, &(_, ref buffer)) in vao.bound_attrib_buffers().borrow().iter() {
                if let Some(ref buffer) = *buffer {
                    buffer.remove_vao_reference(vao.id());
                }
            }
            if let Some(buffer) = vao.bound_buffer_element_array() {
                buffer.remove_vao_reference(vao.id());
            }

            // Delete the vao
            self.ctx.send_command(WebGLCommand::DeleteVertexArray(vao.id()));
            vao.set_deleted();
        }
    }

    // https://www.khronos.org/registry/webgl/extensions/OES_vertex_array_object/
    fn IsVertexArrayOES(&self, vao: Option<&WebGLVertexArrayObjectOES>) -> bool {
        // Conformance tests expect false if vao never bound
        vao.map_or(false, |vao| !vao.is_deleted() && vao.ever_bound())
    }

    // https://www.khronos.org/registry/webgl/extensions/OES_vertex_array_object/
    fn BindVertexArrayOES(&self, vao: Option<&WebGLVertexArrayObjectOES>) {
        if let Some(bound_vao) = self.bound_vao.get() {
            // Store buffers attached to attrib pointers
            bound_vao.bound_attrib_buffers().set_from(&self.ctx.bound_attrib_buffers());
            for (_, (_, ref buffer)) in bound_vao.bound_attrib_buffers().borrow().iter() {
                if let Some(ref buffer) = *buffer {
                    buffer.add_vao_reference(bound_vao.id());
                }
            }
            // Store element array buffer
            let element_array = self.ctx.bound_buffer_element_array();
            bound_vao.set_bound_buffer_element_array(element_array.as_ref().map(|buffer| {
                buffer.add_vao_reference(bound_vao.id());
                &**buffer
            }));
        }

        if let Some(vao) = vao {
            if vao.is_deleted() {
                self.ctx.webgl_error(WebGLError::InvalidOperation);
                return;
            }

            self.ctx.send_command(WebGLCommand::BindVertexArray(Some(vao.id())));
            vao.set_ever_bound();
            self.bound_vao.set(Some(&vao));

            // Restore WebGLRenderingContext current bindings
            self.ctx.bound_attrib_buffers().set_from(&vao.bound_attrib_buffers());
            let element_array = vao.bound_buffer_element_array();
            self.ctx.set_bound_buffer_element_array(element_array.as_ref().map(|buffer| &**buffer));
        } else {
            self.ctx.send_command(WebGLCommand::BindVertexArray(None));
            self.bound_vao.set(None);
            self.ctx.bound_attrib_buffers().clear();
        }
    }
}

impl WebGLExtension for OESVertexArrayObject {
    type Extension = OESVertexArrayObject;
    fn new(ctx: &WebGLRenderingContext) -> DomRoot<OESVertexArrayObject> {
        reflect_dom_object(Box::new(OESVertexArrayObject::new_inherited(ctx)),
                           &*ctx.global(),
                           OESVertexArrayObjectBinding::Wrap)
    }

    fn spec() -> WebGLExtensionSpec {
        WebGLExtensionSpec::Specific(WebGLVersion::WebGL1)
    }

    fn is_supported(ext: &WebGLExtensions) -> bool {
        ext.supports_any_gl_extension(&["GL_OES_vertex_array_object",
                                        "GL_ARB_vertex_array_object",
                                        "GL_APPLE_vertex_array_object"])
    }

    fn enable(ext: &WebGLExtensions) {
        let query = OESVertexArrayObjectConstants::VERTEX_ARRAY_BINDING_OES;
        ext.add_query_parameter_handler(query, Box::new(|cx, webgl_ctx| {
            match webgl_ctx.get_extension_manager().get_dom_object::<OESVertexArrayObject>() {
                Some(dom_object) => {
                    Ok(dom_object.get_current_binding(cx))
                },
                None => {
                    // Extension instance not found!
                    Err(WebGLError::InvalidOperation)
                }
            }
        }));
    }

    fn name() -> &'static str {
        "OES_vertex_array_object"
    }
}

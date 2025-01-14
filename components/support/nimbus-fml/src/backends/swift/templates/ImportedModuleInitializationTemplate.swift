{% import "macros.swift" as swift %}
{%- let variables = "variables" %}
{%- let context = "" %}
{%- let class_name = self.inner.about().nimbus_object_name_swift() %}
        {%- for f in self.inner.features() %}
        {{ class_name }}.shared.features.{{ f.name()|var_name }}.with(initializer: { {{ variables }} in
            {{ f.name()|class_name }}(
                {{ variables }}, {%- for p in f.props() %}
                {{p.name()|var_name}}: {{ p.typ()|literal(self, p.default(), context) }}{% if !loop.last %},{% endif %}
                {%- endfor %}
            )
        })
        {%- endfor %}
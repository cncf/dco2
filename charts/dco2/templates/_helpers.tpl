{{/* vim: set filetype=mustache: */}}
{{/*
Expand the name of the chart.
*/}}
{{- define "dco2.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{/*
Create a fully qualified app name.
*/}}
{{- define "dco2.fullname" -}}
{{- if .Values.fullnameOverride -}}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" -}}
{{- else -}}
{{- $name := include "dco2.name" . -}}
{{- if contains $name .Release.Name -}}
{{- .Release.Name | trunc 63 | trimSuffix "-" -}}
{{- else -}}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" -}}
{{- end -}}
{{- end -}}
{{- end -}}

{{/*
Kubernetes resource name prefix
Using the prefix allows deploying multiple instances of the chart in a single namespace.
If the dynamic prefix is disabled, the helper returns an empty string.
Resource names are capped at 63 characters.
We truncate the prefix to leave space for the longest suffix ("server-config" = 12 chars).
*/}}
{{- define "dco2.resourceNamePrefix" -}}
{{- if .Values.dynamicResourceNamePrefixEnabled -}}
{{- include "dco2.fullname" . | trunc 43 | trimSuffix "-" | printf "%s-" -}}
{{- else -}}
{{- "" -}}
{{- end -}}
{{- end -}}

{{/*
Compose a resource name using the optional prefix.
*/}}
{{- define "dco2.resourceName" -}}
{{- $root := index . 0 -}}
{{- $name := index . 1 -}}
{{- printf "%s%s" (include "dco2.resourceNamePrefix" $root) $name | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{/*
Create chart name and version.
*/}}
{{- define "dco2.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{/*
Common labels.
*/}}
{{- define "dco2.labels" -}}
helm.sh/chart: {{ include "dco2.chart" . }}
{{ include "dco2.selectorLabels" . }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end -}}

{{/*
Selector labels.
*/}}
{{- define "dco2.selectorLabels" -}}
app.kubernetes.io/name: {{ include "dco2.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end -}}

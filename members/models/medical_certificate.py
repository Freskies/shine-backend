from django.db import models
from .member import Member

class MedicalCertificate(models.Model):
    member = models.ForeignKey(Member, on_delete=models.CASCADE, related_name='medical_certificates')
    issue_date = models.DateField()
    expiration_date = models.DateField()
    file = models.FileField(upload_to='medical_certificates/')
    notes = models.TextField(blank=True, null=True)